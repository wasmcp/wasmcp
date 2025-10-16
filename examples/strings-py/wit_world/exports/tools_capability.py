"""
MCP capability interfaces for focused business logic

Capability interfaces provide a clean separation between MCP protocol handling
and domain-specific business logic. Components implementing these interfaces focus
solely on their functionality (tools, prompts, resources) without dealing with:
- JSON-RPC protocol details
- Request routing and delegation
- Response merging across multiple providers
- Error code mapping

Capability components are automatically wrapped by middleware at composition time,
which handles all protocol concerns and integrates them into the server-handler pipeline.

Architecture:
  Client → Transport → [Middleware(Capability₁)] → [Middleware(Capability₂)] → ... → Response

<https://spec.modelcontextprotocol.io>
Tools capability interface

Components implementing this interface provide MCP tools without dealing with
protocol details. The tools-middleware component automatically:
- Wraps this capability as a server-handler
- Merges tools from multiple capabilities
- Handles unknown tool calls by delegating downstream
- Maps errors to appropriate MCP error codes

This interface focuses on two concerns:
1. Listing the tools this component provides
2. Executing tool calls (returning None for unrecognized tools)

<https://spec.modelcontextprotocol.io/specification/server/tools>
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some


