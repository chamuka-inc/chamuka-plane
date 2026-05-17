class SandboxError(Exception):
    def __init__(self, message: str, status_code: int | None = None) -> None:
        super().__init__(message)
        self.status_code = status_code


class TimeoutError(SandboxError):
    def __init__(self, message: str) -> None:
        super().__init__(message)
