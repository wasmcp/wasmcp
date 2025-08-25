/**
 * HTTP client module for WASI
 * 
 * This module provides HTTP client functionality using the native fetch API,
 * which ComponentizeJS maps to WASI HTTP interfaces.
 */

/**
 * HTTP request options
 */
export interface RequestOptions extends RequestInit {
  headers?: Record<string, string>;
}

/**
 * Simplified HTTP response
 */
export interface HttpResponse {
  status: number;
  statusText: string;
  headers: Headers;
  body: string;
  json<T = any>(): T;
}

/**
 * Perform an HTTP request
 * 
 * Uses the native fetch API which is mapped to WASI HTTP by ComponentizeJS
 * 
 * @param url - The URL to request
 * @param options - Request options
 * @returns Promise resolving to the response
 */
export async function request(url: string, options?: RequestOptions): Promise<HttpResponse> {
  const response = await fetch(url, options);
  const body = await response.text();
  
  return {
    status: response.status,
    statusText: response.statusText,
    headers: response.headers,
    body,
    json<T = any>(): T {
      return JSON.parse(body) as T;
    }
  };
}

/**
 * Perform a GET request
 */
export async function get(url: string, options?: RequestOptions): Promise<HttpResponse> {
  return request(url, { ...options, method: 'GET' });
}

/**
 * Perform a POST request
 */
export async function post(url: string, body?: any, options?: RequestOptions): Promise<HttpResponse> {
  const requestBody = typeof body === 'string' ? body : JSON.stringify(body);
  return request(url, {
    ...options,
    method: 'POST',
    body: requestBody,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers
    }
  });
}

/**
 * Perform a PUT request
 */
export async function put(url: string, body?: any, options?: RequestOptions): Promise<HttpResponse> {
  const requestBody = typeof body === 'string' ? body : JSON.stringify(body);
  return request(url, {
    ...options,
    method: 'PUT',
    body: requestBody,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers
    }
  });
}

/**
 * Perform a DELETE request
 */
export async function del(url: string, options?: RequestOptions): Promise<HttpResponse> {
  return request(url, { ...options, method: 'DELETE' });
}

// Note: fetch is globally available in ComponentizeJS runtime
// No need to export it