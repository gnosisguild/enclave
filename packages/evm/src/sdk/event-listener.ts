import { type Abi, type Log, type PublicClient } from "viem";

import {
  type AllEventTypes,
  type EnclaveEvent,
  type EnclaveEventData,
  type EnclaveEventType,
  type EventCallback,
  type EventListenerConfig,
  type RegistryEventData,
  type RegistryEventType,
  type SDKEventEmitter,
} from "./types";
import { SDKError, sleep } from "./utils";

export class EventListener implements SDKEventEmitter {
  private listeners: Map<AllEventTypes, Set<EventCallback>> = new Map();
  private activeWatchers: Map<string, () => void> = new Map();
  private isPolling = false;
  private lastBlockNumber: bigint = BigInt(0);

  constructor(
    private publicClient: PublicClient,
    private config: EventListenerConfig = {},
  ) {}

  /**
   * Listen to specific contract events
   */
  public async watchContractEvent<T extends AllEventTypes>(
    address: `0x${string}`,
    eventType: T,
    abi: Abi,
    callback: EventCallback<T>,
  ): Promise<void> {
    const watcherKey = `${address}:${eventType}`;
    console.log(`watchContractEvent: ${watcherKey}`);

    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }
    console.log("Added callback");
    this.listeners.get(eventType)!.add(callback as EventCallback);

    // If we don't have an active watcher for this event, create one
    if (!this.activeWatchers.has(watcherKey)) {
      console.log("Adding active watcher for " + watcherKey);

      try {
        const unwatch = this.publicClient.watchContractEvent({
          address,
          abi,
          eventName: eventType as string,
          fromBlock: this.config.fromBlock,
          onLogs: (logs: Log[]) => {
            console.log(`Log received for ${watcherKey}`, logs);
            for (let i = 0; i < logs.length; i++) {
              const log = logs[i];
              console.log("Got log!");
              const event: EnclaveEvent<T> = {
                type: eventType,
                data: (log as unknown as { args: unknown })
                  .args as T extends EnclaveEventType
                  ? EnclaveEventData[T]
                  : T extends RegistryEventType
                    ? RegistryEventData[T]
                    : unknown,
                log,
                timestamp: new Date(),
                blockNumber: log.blockNumber ?? BigInt(0),
                transactionHash: log.transactionHash ?? "0x",
              };
              console.log("Created event, now emitting event...");
              this.emit(event);
              console.log("Event emitted");
            }
          },
        });

        this.activeWatchers.set(watcherKey, unwatch);
      } catch (error) {
        throw new SDKError(
          `Failed to watch contract event ${eventType} on ${address}: ${error}`,
          "WATCH_EVENT_FAILED",
        );
      }
    }
  }

  /**
   * Listen to all logs from a specific address
   */
  public async watchLogs(
    address: `0x${string}`,
    callback: (log: Log) => void,
  ): Promise<void> {
    const watcherKey = `logs:${address}`;

    if (!this.activeWatchers.has(watcherKey)) {
      try {
        const unwatch = this.publicClient.watchEvent({
          address,
          onLogs: (logs: Log[]) => {
            logs.forEach((log: Log) => {
              callback(log);
            });
          },
        });

        this.activeWatchers.set(watcherKey, unwatch);
      } catch (error) {
        throw new SDKError(
          `Failed to watch logs for address ${address}: ${error}`,
          "WATCH_LOGS_FAILED",
        );
      }
    }
  }

  /**
   * Start polling for historical events
   */
  public async startPolling(): Promise<void> {
    if (this.isPolling) return;

    this.isPolling = true;

    try {
      this.lastBlockNumber = await this.publicClient.getBlockNumber();

      void this.pollForEvents();
    } catch (error) {
      this.isPolling = false;
      throw new SDKError(
        `Failed to start polling: ${error}`,
        "POLLING_START_FAILED",
      );
    }
  }

  /**
   * Stop polling for events
   */
  public stopPolling(): void {
    this.isPolling = false;
  }

  /**
   * Get historical events
   */
  public async getHistoricalEvents(
    address: `0x${string}`,
    eventType: AllEventTypes,
    abi: Abi,
    fromBlock?: bigint,
    toBlock?: bigint,
  ): Promise<Log[]> {
    try {
      const logs = await this.publicClient.getContractEvents({
        address,
        abi,
        eventName: eventType as string,
        fromBlock: fromBlock || this.config.fromBlock,
        toBlock: toBlock || this.config.toBlock,
      });

      return logs;
    } catch (error) {
      throw new SDKError(
        `Failed to get historical events: ${error}`,
        "HISTORICAL_EVENTS_FAILED",
      );
    }
  }

  /**
   * SDKEventEmitter implementation
   */
  public on<T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>,
  ): void {
    if (!this.listeners.has(eventType)) {
      this.listeners.set(eventType, new Set());
    }
    this.listeners.get(eventType)!.add(callback as EventCallback);
  }

  public off<T extends AllEventTypes>(
    eventType: T,
    callback: EventCallback<T>,
  ): void {
    const callbacks = this.listeners.get(eventType);
    if (callbacks) {
      callbacks.delete(callback as EventCallback);
      if (callbacks.size === 0) {
        this.listeners.delete(eventType);
        // Find and stop corresponding watchers
        const watchersToRemove: string[] = [];
        this.activeWatchers.forEach((unwatch, key) => {
          if (key.endsWith(`:${eventType}`)) {
            try {
              unwatch();
            } catch (error) {
              console.error(`Error unwatching event ${eventType}:`, error);
            }
            watchersToRemove.push(key);
          }
        });
        watchersToRemove.forEach((key) => this.activeWatchers.delete(key));
      }
    }
  }

  public emit<T extends AllEventTypes>(event: EnclaveEvent<T>): void {
    console.log("emit() called with " + JSON.stringify(event));
    const callbacks = this.listeners.get(event.type);
    if (callbacks) {
      callbacks.forEach((callback) => {
        try {
          void (callback as EventCallback<T>)(event);
        } catch (error) {
          console.error(`Error in event callback for ${event.type}:`, error);
        }
      });
    }
  }

  /**
   * Clean up all listeners and watchers
   */
  public cleanup(): void {
    this.stopPolling();

    // Stop all active watchers
    this.activeWatchers.forEach((unwatch) => {
      try {
        unwatch();
      } catch (error) {
        console.error("Error unwatching during cleanup:", error);
      }
    });
    this.activeWatchers.clear();

    // Clear all listeners
    this.listeners.clear();
  }

  private async pollForEvents(): Promise<void> {
    while (this.isPolling) {
      try {
        const currentBlock = await this.publicClient.getBlockNumber();

        if (currentBlock > this.lastBlockNumber) {
          this.lastBlockNumber = currentBlock;
        }

        await sleep(this.config.pollingInterval || 5000);
      } catch (error) {
        console.error("Error during polling:", error);
        await sleep(this.config.pollingInterval || 5000);
      }
    }
  }
}
