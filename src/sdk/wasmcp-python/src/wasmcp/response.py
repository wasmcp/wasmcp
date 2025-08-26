"""Response handling for MCP protocol."""

from enum import IntEnum
from typing import Any, Dict, Optional, Union


class ErrorCodes(IntEnum):
    """Standard JSON-RPC error codes used in MCP."""
    
    PARSE_ERROR = -32700
    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_PARAMS = -32602
    INTERNAL_ERROR = -32603


class McpResponse:
    """Response builder for MCP protocol messages."""
    
    @staticmethod
    def success(result: Any) -> Dict[str, Any]:
        """Create a successful response.
        
        Args:
            result: The result data
            
        Returns:
            MCP success response
        """
        return {"result": result}
    
    @staticmethod
    def error(
        code: int,
        message: str,
        data: Optional[Any] = None
    ) -> Dict[str, Any]:
        """Create an error response.
        
        Args:
            code: Error code
            message: Error message
            data: Optional error data
            
        Returns:
            MCP error response
        """
        error_obj = {"code": code, "message": message}
        if data is not None:
            error_obj["data"] = data
        return {"error": error_obj}
    
    @staticmethod
    def method_not_found(method: str) -> Dict[str, Any]:
        """Create method not found error.
        
        Args:
            method: The method that was not found
            
        Returns:
            Method not found error response
        """
        return McpResponse.error(
            ErrorCodes.METHOD_NOT_FOUND,
            f"Method not found: {method}"
        )
    
    @staticmethod
    def internal_error(message: str, data: Optional[Any] = None) -> Dict[str, Any]:
        """Create internal error response.
        
        Args:
            message: Error message
            data: Optional error data
            
        Returns:
            Internal error response
        """
        return McpResponse.error(ErrorCodes.INTERNAL_ERROR, message, data)
    
    @staticmethod
    def invalid_params(message: str) -> Dict[str, Any]:
        """Create invalid params error.
        
        Args:
            message: Error message
            
        Returns:
            Invalid params error response
        """
        return McpResponse.error(ErrorCodes.INVALID_PARAMS, message)