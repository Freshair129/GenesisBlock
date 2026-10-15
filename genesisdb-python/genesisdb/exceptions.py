class GenesisError(Exception):
    """Base error for GenesisBlockDB SDK"""
    pass

class ConnectionError(GenesisError):
    """Failed to connect to server"""
    pass

class QueryError(GenesisError):
    """Server returned an error for the query"""

    def __init__(
        self,
        message: str,
        *,
        status_code: int = 0,
        code: str = "QUERY_EXECUTION_FAILED",
        category: str = None,
        stable_code: str = None,
        legacy_code: str = None,
    ):
        super().__init__(message)
        self.status_code = status_code
        self.code = code
        self.stable_code = stable_code
        self.legacy_code = legacy_code
        self.message = message
        self.category = category
