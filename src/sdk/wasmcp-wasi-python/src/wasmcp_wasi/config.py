"""Configuration access for WASI environments using Spin SDK."""

from typing import Dict, List, Optional, Tuple

from spin_sdk import variables as spin_config


def get(key: str) -> Optional[str]:
    """Get a configuration value by key.
    
    Args:
        key: Configuration key
        
    Returns:
        Configuration value or None if not found
        
    Raises:
        Exception: If access fails
    """
    try:
        return spin_config.get(key)
    except Exception as e:
        # Spin raises an exception if key not found
        # Check if it's a "not found" error or a real error
        if "not found" in str(e).lower():
            return None
        raise


def get_all() -> Dict[str, str]:
    """Get all configuration values.
    
    Returns:
        Dictionary of all configuration key-value pairs
        
    Raises:
        Exception: If access fails
    """
    # Note: Spin SDK doesn't have a direct get_all method
    # This would need to be implemented based on available configs
    # For now, return empty dict as placeholder
    # In practice, you'd need to know which keys to query
    return {}


def require(key: str) -> str:
    """Get a required configuration value.
    
    Args:
        key: Configuration key
        
    Returns:
        Configuration value
        
    Raises:
        ValueError: If key not found
        Exception: If access fails
    """
    value = get(key)
    if value is None:
        raise ValueError(f"Required configuration key not found: {key}")
    return value


def get_with_default(key: str, default: str) -> str:
    """Get a configuration value with a default.
    
    Args:
        key: Configuration key
        default: Default value if key not found
        
    Returns:
        Configuration value or default
        
    Raises:
        Exception: If access fails
    """
    value = get(key)
    return value if value is not None else default