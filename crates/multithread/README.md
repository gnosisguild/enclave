# e3-multithread

Shared Rayon task admission and ciphernode compute dispatch for ZK and threshold-BFV work. It keeps
expensive jobs off actor mailboxes and coordinates replay-safe effects; protocol validity remains
with the requesting capabilities.

## Task pool

`TaskPool::new(threads, max_tasks)` separates worker-thread count from concurrent job capacity. Its
async `spawn(task_name, timed_logs, operation)` waits for admission and returns the operation's
result (or `TaskPoolError`). Timed logs describe elapsed work; they are not hard preemption of
executing Rayon closures.

For production sizing, `ComputeCapacity::detect(requested_jobs, cpu_jobs)` applies CPU and detected
host/cgroup-memory limits. [CiphernodeBuilder](../ciphernode-builder/README.md) exposes
`with_multithread_config` and optional shared task pools/reports. ZK and TrBFV work share admission
instead of each using unrelated pool capacity.

## Event-driven workers

`Multithread::attach` installs threshold-BFV dispatch. `attach_with_zk` additionally supplies the
prover backend, restored lifecycle stages, and canonical keys. Both need the bus, RNG, local cipher,
task pool, and node-specific task scope; see the
[builder caller](../ciphernode-builder/src/builder/extensions.rs) for the complete wiring. These are
node-wide workers, not per-E3 router extensions.

Dispatch correlates jobs with their request/E3 and gates recovery on current-boot effect readiness.
Queued cancellation does not stop an executing closure. Prover process deadlines are separate, and
local resource failure is not proof that a peer supplied invalid crypto. `MultithreadReport` /
`ToReport` expose timing and capacity diagnostics, not proof validity.

See
[effect recovery](../../docs/architecture/persistence-and-replay.md#effect-recovery-and-shutdown)
for admission/retry limits. `cargo test --locked -p e3-multithread` transitively builds the prover;
read its [build prerequisites and side effects](../zk-prover/README.md) first.
