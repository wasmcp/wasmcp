"""Tests for configuration module."""

import pytest
from unittest.mock import patch, Mock
from wasmcp_wasi import config


class TestConfig:
    """Test configuration functions."""
    
    @patch('spin_sdk.variables.get')
    def test_get_existing_value(self, mock_spin_get):
        """Test getting an existing configuration value."""
        mock_spin_get.return_value = "test_value"
        
        result = config.get("TEST_KEY")
        assert result == "test_value"
        mock_spin_get.assert_called_once_with("TEST_KEY")
    
    @patch('spin_sdk.variables.get')
    def test_get_nonexistent_value(self, mock_spin_get):
        """Test getting a non-existent configuration value."""
        # Simulate Spin SDK throwing exception for missing key
        mock_spin_get.side_effect = Exception("Key not found")
        
        result = config.get("MISSING_KEY")
        assert result is None
        mock_spin_get.assert_called_once_with("MISSING_KEY")
    
    @patch('spin_sdk.variables.get')
    def test_get_real_error(self, mock_spin_get):
        """Test real error during configuration access."""
        # Simulate a real error (not just missing key)
        mock_spin_get.side_effect = Exception("Permission denied")
        
        with pytest.raises(Exception, match="Permission denied"):
            config.get("ANY_KEY")
    
    def test_get_all(self):
        """Test getting all configuration values."""
        # Note: Current implementation returns empty dict as placeholder
        # This test documents the current behavior
        result = config.get_all()
        assert isinstance(result, dict)
        assert len(result) == 0
    
    @patch('spin_sdk.variables.get')
    def test_require_existing_value(self, mock_spin_get):
        """Test requiring an existing configuration value."""
        mock_spin_get.return_value = "required_value"
        
        result = config.require("REQUIRED_KEY")
        assert result == "required_value"
        mock_spin_get.assert_called_once_with("REQUIRED_KEY")
    
    @patch('spin_sdk.variables.get')
    def test_require_missing_value(self, mock_spin_get):
        """Test requiring a missing configuration value."""
        mock_spin_get.side_effect = Exception("Key not found")
        
        with pytest.raises(ValueError, match="Required configuration key not found: MISSING_KEY"):
            config.require("MISSING_KEY")
    
    @patch('spin_sdk.variables.get')
    def test_get_with_default_existing(self, mock_spin_get):
        """Test getting value with default when value exists."""
        mock_spin_get.return_value = "actual_value"
        
        result = config.get_with_default("EXISTING_KEY", "default_value")
        assert result == "actual_value"
        mock_spin_get.assert_called_once_with("EXISTING_KEY")
    
    @patch('spin_sdk.variables.get')
    def test_get_with_default_missing(self, mock_spin_get):
        """Test getting value with default when value is missing."""
        mock_spin_get.side_effect = Exception("Key not found")
        
        result = config.get_with_default("MISSING_KEY", "default_value")
        assert result == "default_value"
        mock_spin_get.assert_called_once_with("MISSING_KEY")
    
    @patch('spin_sdk.variables.get')
    def test_get_with_default_error(self, mock_spin_get):
        """Test getting value with default when there's an error."""
        mock_spin_get.side_effect = Exception("Access denied")
        
        with pytest.raises(Exception, match="Access denied"):
            config.get_with_default("ERROR_KEY", "default")
    
    @patch('spin_sdk.variables.get')
    def test_multiple_config_calls(self, mock_spin_get):
        """Test multiple configuration calls."""
        # Set up different return values
        mock_spin_get.side_effect = [
            "value1",
            Exception("Key not found"),
            "value3"
        ]
        
        assert config.get("KEY1") == "value1"
        assert config.get("KEY2") is None
        assert config.get("KEY3") == "value3"
        
        assert mock_spin_get.call_count == 3