export class IdlError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'IdlError';
  }
}
