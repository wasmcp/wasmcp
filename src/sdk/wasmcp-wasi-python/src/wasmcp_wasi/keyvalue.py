"""Key-value storage for WASI environments using Spin SDK.

This module provides key-value storage capabilities when running
in Spin environments that support the WASI KV interface.
"""

import json
from typing import Any, Dict, List, Optional, Tuple, Union

try:
    from spin_sdk import key_value
    KEYVALUE_AVAILABLE = True
except ImportError:
    KEYVALUE_AVAILABLE = False
    key_value = None  # type: ignore


class Store:
    """A key-value store wrapper for Spin SDK."""
    
    def __init__(self, name: str = "default"):
        """Open a key-value store.
        
        Args:
            name: Store name (default: "default")
            
        Raises:
            RuntimeError: If key-value support is not available
            Exception: If store cannot be opened
        """
        if not KEYVALUE_AVAILABLE:
            raise RuntimeError(
                "Key-value storage not available. "
                "Ensure you're running in Spin with KV support enabled."
            )
        
        self.name = name
        self._store = key_value.open(name)
    
    def get(self, key: str) -> Optional[bytes]:
        """Get a value by key.
        
        Args:
            key: Key to retrieve
            
        Returns:
            Value as bytes or None if not found
            
        Raises:
            Exception: If retrieval fails
        """
        try:
            return self._store.get(key)
        except Exception as e:
            if "not found" in str(e).lower():
                return None
            raise
    
    def get_str(self, key: str) -> Optional[str]:
        """Get a string value by key.
        
        Args:
            key: Key to retrieve
            
        Returns:
            Value as string or None if not found
            
        Raises:
            Exception: If retrieval fails
        """
        value = self.get(key)
        return value.decode('utf-8') if value else None
    
    def get_json(self, key: str) -> Optional[Any]:
        """Get a JSON value by key.
        
        Args:
            key: Key to retrieve
            
        Returns:
            Parsed JSON value or None if not found
            
        Raises:
            Exception: If retrieval fails
            json.JSONDecodeError: If value is not valid JSON
        """
        value = self.get_str(key)
        return json.loads(value) if value else None
    
    def set(self, key: str, value: Union[bytes, str, dict, list]) -> None:
        """Set a key-value pair.
        
        Args:
            key: Key to set
            value: Value to store (bytes, string, or JSON-serializable)
            
        Raises:
            Exception: If storage fails
        """
        if isinstance(value, bytes):
            data = value
        elif isinstance(value, str):
            data = value.encode('utf-8')
        else:
            # Assume JSON-serializable
            data = json.dumps(value).encode('utf-8')
        
        self._store.set(key, data)
    
    def delete(self, key: str) -> bool:
        """Delete a key.
        
        Args:
            key: Key to delete
            
        Returns:
            True if key was deleted, False if not found
            
        Raises:
            Exception: If deletion fails
        """
        try:
            self._store.delete(key)
            return True
        except Exception as e:
            if "not found" in str(e).lower():
                return False
            raise
    
    def exists(self, key: str) -> bool:
        """Check if a key exists.
        
        Args:
            key: Key to check
            
        Returns:
            True if key exists
            
        Raises:
            Exception: If check fails
        """
        return self._store.exists(key)
    
    def list_keys(self, prefix: Optional[str] = None) -> List[str]:
        """List all keys, optionally with a prefix filter.
        
        Args:
            prefix: Optional prefix to filter keys
            
        Returns:
            List of keys
            
        Raises:
            Exception: If listing fails
        """
        # Note: Spin SDK's key_value.get_keys() returns all keys
        # We need to filter by prefix manually
        all_keys = self._store.get_keys()
        
        if prefix:
            return [k for k in all_keys if k.startswith(prefix)]
        return all_keys
    
    def clear(self, prefix: Optional[str] = None) -> int:
        """Clear all keys, optionally with a prefix filter.
        
        Args:
            prefix: Optional prefix to filter keys to delete
            
        Returns:
            Number of keys deleted
            
        Raises:
            Exception: If deletion fails
        """
        keys = self.list_keys(prefix)
        count = 0
        
        for key in keys:
            if self.delete(key):
                count += 1
        
        return count
    
    def get_many(self, keys: List[str]) -> Dict[str, Optional[bytes]]:
        """Get multiple values by keys.
        
        Args:
            keys: List of keys to retrieve
            
        Returns:
            Dictionary mapping keys to values (None if not found)
            
        Raises:
            Exception: If retrieval fails
        """
        result = {}
        for key in keys:
            result[key] = self.get(key)
        return result
    
    def set_many(self, items: Dict[str, Union[bytes, str, dict, list]]) -> None:
        """Set multiple key-value pairs.
        
        Args:
            items: Dictionary of key-value pairs
            
        Raises:
            Exception: If storage fails
        """
        for key, value in items.items():
            self.set(key, value)


def open(name: str = "default") -> Store:
    """Open a key-value store.
    
    Args:
        name: Store name (default: "default")
        
    Returns:
        Store instance
        
    Raises:
        RuntimeError: If key-value support is not available
        Exception: If store cannot be opened
    """
    return Store(name)


def is_available() -> bool:
    """Check if key-value storage is available.
    
    Returns:
        True if KV storage is available
    """
    return KEYVALUE_AVAILABLE