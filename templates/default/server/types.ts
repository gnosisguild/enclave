import { AllEventTypes } from "@enclave-e3/sdk";

export enum ProgramEventType {
    INPUT_PUBLISHED = "InputPublished"
}

export type ProgramEvents = ProgramEventType | AllEventTypes;

export interface InputPublishedEvent {
    e3Id: bigint
    data: string;
    index: bigint;
}
