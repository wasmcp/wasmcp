"""
Auto-patching module that automatically patches standard libraries on import.

Simply import this module to make urllib and http.client work transparently:

    import wasi_http_async.auto_patch
    
    # Now these all work
    import urllib.request
    response = urllib.request.urlopen('https://api.example.com/data')
"""

from .compat import patch_urllib, install_fetch

# Automatically patch on import
patch_urllib()
install_fetch()

print("wasi-http-async: urllib and fetch() have been patched for WASI HTTP support")