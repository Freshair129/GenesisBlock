class GenesisError(Exception):
    """Base error for GenesisDB SDK"""
    pass

class ConnectionError(GenesisError):
    """Failed to connect to server"""
    pass

class QueryError(GenesisError):
    """Server returned an error for the query"""
    pass
