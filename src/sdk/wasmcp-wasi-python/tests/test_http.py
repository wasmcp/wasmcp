"""Tests for HTTP client module."""

import pytest
import json
from unittest.mock import Mock, patch, MagicMock
from wasmcp_wasi.http import (
    HttpMethod, Request, Response, send, get, post, put, delete, patch as http_patch
)


class TestHttpMethod:
    """Test HttpMethod enum."""
    
    def test_http_methods(self):
        """Test HTTP method enum values."""
        assert HttpMethod.GET.value == "GET"
        assert HttpMethod.POST.value == "POST"
        assert HttpMethod.PUT.value == "PUT"
        assert HttpMethod.DELETE.value == "DELETE"
        assert HttpMethod.PATCH.value == "PATCH"
        assert HttpMethod.HEAD.value == "HEAD"
        assert HttpMethod.OPTIONS.value == "OPTIONS"


class TestRequest:
    """Test Request class."""
    
    def test_basic_request_creation(self):
        """Test creating a basic request."""
        req = Request("https://example.com")
        assert req.url == "https://example.com"
        assert req.method == HttpMethod.GET
        assert req.headers == {}
        assert req.body is None
    
    def test_request_with_method(self):
        """Test request with custom method."""
        req = Request("https://example.com", HttpMethod.POST)
        assert req.method == HttpMethod.POST
        
        # Test with string method
        req = Request("https://example.com", "PUT")
        assert req.method == HttpMethod.PUT
    
    def test_request_with_headers(self):
        """Test request with headers."""
        headers = {"Authorization": "Bearer token", "Content-Type": "application/json"}
        req = Request("https://example.com", headers=headers)
        assert req.headers == headers
    
    def test_prepare_body_none(self):
        """Test preparing None body."""
        req = Request("https://example.com")
        assert req._prepare_body() is None
    
    def test_prepare_body_bytes(self):
        """Test preparing bytes body."""
        req = Request("https://example.com", body=b"binary data")
        assert req._prepare_body() == b"binary data"
    
    def test_prepare_body_string(self):
        """Test preparing string body."""
        req = Request("https://example.com", body="text data")
        assert req._prepare_body() == b"text data"
    
    def test_prepare_body_dict(self):
        """Test preparing dict body as JSON."""
        data = {"key": "value", "number": 42}
        req = Request("https://example.com", body=data)
        
        body = req._prepare_body()
        assert json.loads(body) == data
        assert req.headers["Content-Type"] == "application/json"
    
    def test_prepare_body_list(self):
        """Test preparing list body as JSON."""
        data = [1, 2, 3]
        req = Request("https://example.com", body=data)
        
        body = req._prepare_body()
        assert json.loads(body) == data
        assert req.headers["Content-Type"] == "application/json"
    
    def test_to_wasi_request(self):
        """Test conversion to WASI request."""        
        req = Request(
            "https://example.com/api",
            HttpMethod.POST,
            headers={"Auth": "token"},
            body="test"
        )
        
        wasi_request = req.to_wasi_request()
        
        assert wasi_request["method"] == "POST"
        assert wasi_request["uri"] == "https://example.com/api"
        assert wasi_request["headers"] == [("Auth", "token")]
        assert wasi_request["body"] == b"test"


class TestResponse:
    """Test Response class."""
    
    def test_response_creation(self):
        """Test creating a response."""
        response = Response(
            status=200,
            headers={"Content-Type": "application/json"},
            body=b'{"result": "success"}'
        )
        assert response.status == 200
        assert response.headers == {"Content-Type": "application/json"}
    
    def test_response_body(self):
        """Test getting response body."""
        response = Response(
            status=200,
            headers={},
            body=b"test body"
        )
        assert response.body == b"test body"
        
        # Test caching
        assert response.body is response.body
    
    def test_response_text(self):
        """Test getting response as text."""
        response = Response(
            status=200,
            headers={},
            body=b"text content"
        )
        assert response.text() == "text content"
        
        # Test caching
        first_call = response.text()
        second_call = response.text()
        assert first_call is second_call
    
    def test_response_json(self):
        """Test parsing response as JSON."""
        response = Response(
            status=200,
            headers={},
            body=b'{"key": "value", "number": 42}'
        )
        data = response.json()
        assert data == {"key": "value", "number": 42}
        
        # Test caching
        assert response.json() is response.json()
    
    def test_response_json_error(self):
        """Test JSON parsing error."""
        response = Response(
            status=200,
            headers={},
            body=b"not json"
        )
        with pytest.raises(json.JSONDecodeError):
            response.json()
    
    def test_response_ok_property(self):
        """Test ok property for different status codes."""
        # 2xx status codes are ok
        for status in [200, 201, 204, 299]:
            response = Response(status, {}, b"")
            assert response.ok
        
        # Non-2xx status codes are not ok
        for status in [100, 199, 300, 400, 404, 500]:
            response = Response(status, {}, b"")
            assert not response.ok
    
    def test_response_empty_body(self):
        """Test response with empty body."""
        response = Response(
            status=204,
            headers={},
            body=b""
        )
        assert response.body == b""
        assert response.text() == ""


class TestHttpFunctions:
    """Test module-level HTTP functions."""
    
    @patch('wasmcp_wasi.http.send')
    def test_get_function(self, mock_send):
        """Test get() function."""
        mock_response = Mock()
        mock_send.return_value = mock_response
        
        result = get("https://example.com", headers={"Auth": "token"})
        
        assert result == mock_response
        mock_send.assert_called_once()
        
        # Check the request that was created
        call_args = mock_send.call_args
        request = call_args[0][0]
        assert isinstance(request, Request)
        assert request.url == "https://example.com"
        assert request.method == HttpMethod.GET
        assert request.headers == {"Auth": "token"}
    
    @patch('wasmcp_wasi.http.send')
    def test_post_function(self, mock_send):
        """Test post() function."""
        mock_response = Mock()
        mock_send.return_value = mock_response
        
        body_data = {"key": "value"}
        result = post(
            "https://example.com/api",
            headers={"Auth": "token"},
            body=body_data
        )
        
        assert result == mock_response
        mock_send.assert_called_once()
        
        request = mock_send.call_args[0][0]
        assert request.url == "https://example.com/api"
        assert request.method == HttpMethod.POST
        assert request.body == body_data
    
    @patch('wasmcp_wasi.http.send')
    def test_put_function(self, mock_send):
        """Test put() function."""
        mock_response = Mock()
        mock_send.return_value = mock_response
        
        result = put("https://example.com", body="data")
        
        assert result == mock_response
        request = mock_send.call_args[0][0]
        assert request.method == HttpMethod.PUT
        assert request.body == "data"
    
    @patch('wasmcp_wasi.http.send')
    def test_delete_function(self, mock_send):
        """Test delete() function."""
        mock_response = Mock()
        mock_send.return_value = mock_response
        
        result = delete("https://example.com/resource")
        
        assert result == mock_response
        request = mock_send.call_args[0][0]
        assert request.method == HttpMethod.DELETE
        assert request.url == "https://example.com/resource"
    
    @patch('wasmcp_wasi.http.send')
    def test_patch_function(self, mock_send):
        """Test patch() function."""
        mock_response = Mock()
        mock_send.return_value = mock_response
        
        result = http_patch("https://example.com", body={"update": "value"})
        
        assert result == mock_response
        request = mock_send.call_args[0][0]
        assert request.method == HttpMethod.PATCH
        assert request.body == {"update": "value"}
    
    @patch('urllib.request.urlopen')
    def test_send_function(self, mock_urlopen):
        """Test send() function."""
        mock_urllib_response = Mock()
        mock_urllib_response.getcode.return_value = 200
        mock_urllib_response.headers = {}
        mock_urllib_response.read.return_value = b"response"
        mock_urlopen.return_value = mock_urllib_response
        
        request = Request("https://example.com", HttpMethod.GET)
        response = send(request)
        
        assert isinstance(response, Response)
        assert response.status == 200
        assert response.body == b"response"
        
        mock_urlopen.assert_called_once()