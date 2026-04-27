export class UserError extends Error {
  readonly code = 2 as const;
  constructor(message: string) {
    super(message);
    this.name = 'UserError';
  }
}
