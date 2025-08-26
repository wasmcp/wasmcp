"""Tests for key-value storage module."""

import pytest
import json
from unittest.mock import Mock, patch, MagicMock
from wasmcp_wasi import keyvalue


class TestStore:
    """Test Store class."""
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_store_creation(self, mock_kv):
        """Test creating a Store."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store("test-store")
        assert store.name == "test-store"
        assert store._store == mock_store_obj
        mock_kv.open.assert_called_once_with("test-store")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_store_default_name(self, mock_kv):
        """Test creating a Store with default name."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        assert store.name == "default"
        mock_kv.open.assert_called_once_with("default")
    
    @patch('wasmcp_wasi.keyvalue.KEYVALUE_AVAILABLE', False)
    def test_store_not_available(self):
        """Test creating Store when KV is not available."""
        with pytest.raises(RuntimeError, match="Key-value storage not available"):
            keyvalue.Store()
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_get_existing_value(self, mock_kv):
        """Test getting an existing value."""
        mock_store_obj = Mock()
        mock_store_obj.get.return_value = b"test_value"
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.get("test_key")
        assert result == b"test_value"
        mock_store_obj.get.assert_called_once_with("test_key")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_get_nonexistent_value(self, mock_kv):
        """Test getting a non-existent value."""
        mock_store_obj = Mock()
        mock_store_obj.get.side_effect = Exception("Key not found")
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.get("missing_key")
        assert result is None
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_get_str(self, mock_kv):
        """Test getting a string value."""
        mock_store_obj = Mock()
        mock_store_obj.get.return_value = b"hello world"
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.get_str("string_key")
        assert result == "hello world"
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_get_str_none(self, mock_kv):
        """Test getting None when key doesn't exist."""
        mock_store_obj = Mock()
        mock_store_obj.get.side_effect = Exception("Key not found")
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.get_str("missing")
        assert result is None
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_get_json(self, mock_kv):
        """Test getting a JSON value."""
        data = {"key": "value", "number": 42}
        mock_store_obj = Mock()
        mock_store_obj.get.return_value = json.dumps(data).encode('utf-8')
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.get_json("json_key")
        assert result == data
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_set_bytes(self, mock_kv):
        """Test setting bytes value."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        store.set("key", b"binary_data")
        mock_store_obj.set.assert_called_once_with("key", b"binary_data")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_set_string(self, mock_kv):
        """Test setting string value."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        store.set("key", "string_value")
        mock_store_obj.set.assert_called_once_with("key", b"string_value")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_set_dict(self, mock_kv):
        """Test setting dict value as JSON."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        data = {"a": 1, "b": 2}
        store.set("key", data)
        
        expected_bytes = json.dumps(data).encode('utf-8')
        mock_store_obj.set.assert_called_once_with("key", expected_bytes)
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_set_list(self, mock_kv):
        """Test setting list value as JSON."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        data = [1, 2, 3]
        store.set("key", data)
        
        expected_bytes = json.dumps(data).encode('utf-8')
        mock_store_obj.set.assert_called_once_with("key", expected_bytes)
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_delete_existing(self, mock_kv):
        """Test deleting an existing key."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.delete("existing_key")
        assert result is True
        mock_store_obj.delete.assert_called_once_with("existing_key")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_delete_nonexistent(self, mock_kv):
        """Test deleting a non-existent key."""
        mock_store_obj = Mock()
        mock_store_obj.delete.side_effect = Exception("Key not found")
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.delete("missing_key")
        assert result is False
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_exists_true(self, mock_kv):
        """Test checking if key exists (true case)."""
        mock_store_obj = Mock()
        mock_store_obj.exists.return_value = True
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        assert store.exists("key") is True
        mock_store_obj.exists.assert_called_once_with("key")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_exists_false(self, mock_kv):
        """Test checking if key exists (false case)."""
        mock_store_obj = Mock()
        mock_store_obj.exists.return_value = False
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        assert store.exists("key") is False
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_list_keys(self, mock_kv):
        """Test listing all keys."""
        mock_store_obj = Mock()
        mock_store_obj.get_keys.return_value = ["key1", "key2", "key3"]
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        keys = store.list_keys()
        assert keys == ["key1", "key2", "key3"]
        mock_store_obj.get_keys.assert_called_once()
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_list_keys_with_prefix(self, mock_kv):
        """Test listing keys with prefix filter."""
        mock_store_obj = Mock()
        mock_store_obj.get_keys.return_value = ["user:1", "user:2", "config:app", "user:3"]
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        keys = store.list_keys("user:")
        assert keys == ["user:1", "user:2", "user:3"]
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_clear_all(self, mock_kv):
        """Test clearing all keys."""
        mock_store_obj = Mock()
        mock_store_obj.get_keys.return_value = ["key1", "key2", "key3"]
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        count = store.clear()
        
        assert count == 3
        assert mock_store_obj.delete.call_count == 3
        mock_store_obj.delete.assert_any_call("key1")
        mock_store_obj.delete.assert_any_call("key2")
        mock_store_obj.delete.assert_any_call("key3")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_clear_with_prefix(self, mock_kv):
        """Test clearing keys with prefix."""
        mock_store_obj = Mock()
        mock_store_obj.get_keys.return_value = ["temp:1", "temp:2", "keep:1"]
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        count = store.clear("temp:")
        
        assert count == 2
        assert mock_store_obj.delete.call_count == 2
        mock_store_obj.delete.assert_any_call("temp:1")
        mock_store_obj.delete.assert_any_call("temp:2")
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_get_many(self, mock_kv):
        """Test getting multiple values."""
        mock_store_obj = Mock()
        mock_store_obj.get.side_effect = [
            b"value1",
            Exception("Key not found"),
            b"value3"
        ]
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        result = store.get_many(["key1", "key2", "key3"])
        
        assert result == {
            "key1": b"value1",
            "key2": None,
            "key3": b"value3"
        }
    
    @patch('wasmcp_wasi.keyvalue.key_value')
    def test_set_many(self, mock_kv):
        """Test setting multiple values."""
        mock_store_obj = Mock()
        mock_kv.open.return_value = mock_store_obj
        
        store = keyvalue.Store()
        store.set_many({
            "key1": "string",
            "key2": {"dict": "value"},
            "key3": b"bytes"
        })
        
        assert mock_store_obj.set.call_count == 3
        mock_store_obj.set.assert_any_call("key1", b"string")
        mock_store_obj.set.assert_any_call("key2", json.dumps({"dict": "value"}).encode())
        mock_store_obj.set.assert_any_call("key3", b"bytes")


class TestModuleFunctions:
    """Test module-level functions."""
    
    @patch('wasmcp_wasi.keyvalue.Store')
    def test_open_function(self, mock_store_class):
        """Test open() function."""
        mock_store = Mock()
        mock_store_class.return_value = mock_store
        
        result = keyvalue.open("my-store")
        assert result == mock_store
        mock_store_class.assert_called_once_with("my-store")
    
    @patch('wasmcp_wasi.keyvalue.Store')
    def test_open_default(self, mock_store_class):
        """Test open() with default name."""
        mock_store = Mock()
        mock_store_class.return_value = mock_store
        
        result = keyvalue.open()
        assert result == mock_store
        mock_store_class.assert_called_once_with("default")
    
    @patch('wasmcp_wasi.keyvalue.KEYVALUE_AVAILABLE', True)
    def test_is_available_true(self):
        """Test is_available() when KV is available."""
        assert keyvalue.is_available() is True
    
    @patch('wasmcp_wasi.keyvalue.KEYVALUE_AVAILABLE', False)
    def test_is_available_false(self):
        """Test is_available() when KV is not available."""
        assert keyvalue.is_available() is False