export class SandboxError extends Error {
  constructor(
    message: string,
    public readonly statusCode?: number,
    public readonly code?: string,
  ) {
    super(message);
    this.name = "SandboxError";
  }
}

export class TimeoutError extends SandboxError {
  constructor(message: string) {
    super(message, undefined, "TIMEOUT");
    this.name = "TimeoutError";
  }
}

export class ConnectionError extends SandboxError {
  constructor(message: string) {
    super(message, undefined, "CONNECTION_ERROR");
    this.name = "ConnectionError";
  }
}
