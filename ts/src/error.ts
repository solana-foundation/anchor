export class IdlError extends Error {}

// An error from a user defined program.
export class ProgramError extends Error {
  constructor(readonly code: number, readonly msg: string, ...params: any[]) {
    super(...params);
  }

  public toString(): string {
    return this.msg;
  }
}
