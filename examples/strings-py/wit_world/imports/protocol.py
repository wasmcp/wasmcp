"""
Model Context Protocol (MCP) core types and messages

This interface defines all MCP protocol types, including:
- JSON-RPC message structures (requests, responses, notifications, errors)
- Content types (text, images, audio, resources)
- Server capabilities (tools, prompts, resources)
- Client capabilities (sampling, roots, elicitation)
- Session and context management

These types form the foundation for MCP communication between clients and servers.
They are used by both handler interfaces and capability interfaces.

Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18>
"""
from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..imports import streams


@dataclass
class TextData_Text:
    value: str


@dataclass
class TextData_TextStream:
    value: streams.InputStream


TextData = Union[TextData_Text, TextData_TextStream]
"""
=========================================================================
Streaming Data Types
=========================================================================
Text data that can be provided as a string or stream

Streaming text is useful for large content that shouldn't be
buffered entirely in memory.
"""



@dataclass
class BlobData_Blob:
    value: bytes


@dataclass
class BlobData_BlobStream:
    value: streams.InputStream


BlobData = Union[BlobData_Blob, BlobData_BlobStream]
"""
Binary data that can be provided as bytes or stream

Streaming blobs is useful for large binary content (images, audio, etc.)
that shouldn't be buffered entirely in memory.
"""


@dataclass
class EmbeddedResourceOptions:
    """
    =========================================================================
    Content Types and Options
    =========================================================================
    Options for embedded resources
    """
    mime_type: Optional[str]
    meta: Optional[str]

@dataclass
class TextResourceContents:
    """
    Text resource contents
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#textresourcecontents>
    """
    uri: str
    text: TextData
    options: Optional[EmbeddedResourceOptions]

@dataclass
class BlobResourceContents:
    """
    Binary resource contents
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#blobresourcecontents>
    """
    uri: str
    blob: BlobData
    options: Optional[EmbeddedResourceOptions]


@dataclass
class ResourceContents_Text:
    value: TextResourceContents


@dataclass
class ResourceContents_Blob:
    value: BlobResourceContents


ResourceContents = Union[ResourceContents_Text, ResourceContents_Blob]
"""
Resource contents (text or binary)
"""


class LogLevel(Enum):
    """
    =========================================================================
    Protocol Enumerations
    =========================================================================
    Log severity levels
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/server/utilities/logging#log-levels>
    """
    DEBUG = 0
    INFO = 1
    NOTICE = 2
    WARNING = 3
    ERROR = 4
    CRITICAL = 5
    ALERT = 6
    EMERGENCY = 7


@dataclass
class ProgressToken_String:
    value: str


@dataclass
class ProgressToken_Integer:
    value: int


ProgressToken = Union[ProgressToken_String, ProgressToken_Integer]
"""
Progress token for tracking long-running operations

Progress tokens MUST be either a string or integer value.

Spec: <https://spec.modelcontextprotocol.io/specification/2025-03-26/basic/utilities/progress#progress-flow>
"""



@dataclass
class RequestId_Number:
    value: int


@dataclass
class RequestId_String:
    value: str


RequestId = Union[RequestId_Number, RequestId_String]
"""
JSON-RPC request identifier

Request IDs can be either strings or numbers as per JSON-RPC 2.0.

Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#requestid>
"""


class ProtocolVersion(Enum):
    """
    MCP protocol version
    
    The protocol version determines which features and message types are available.
    """
    V20250618 = 0
    V20250326 = 1
    V20241105 = 2

class ServerLists(Flag):
    """
    Server capability list change flags
    
    Used in notifications to indicate which server lists have changed.
    """
    TOOLS = auto()
    RESOURCES = auto()
    PROMPTS = auto()

class ServerSubscriptions(Flag):
    """
    Server subscription type flags
    
    Indicates which subscription types the server supports.
    """
    RESOURCES = auto()

class ClientLists(Flag):
    """
    Client capability list change flags
    
    Used in notifications to indicate which client lists have changed.
    """
    ROOTS = auto()

class Role(Enum):
    """
    Role in a conversation
    
    Identifies whether a message is from the user or assistant.
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#role>
    """
    USER = 0
    ASSISTANT = 1

@dataclass
class ServerCapabilities:
    """
    =========================================================================
    Capability Structures
    =========================================================================
    Server capabilities advertised during initialization
    
    Indicates which optional features the server supports.
    """
    completions: Optional[str]
    experimental: Optional[List[Tuple[str, str]]]
    logging: Optional[str]
    list_changed: Optional[ServerLists]
    subscriptions: Optional[ServerSubscriptions]

@dataclass
class ClientCapabilities:
    """
    Client capabilities advertised during initialization
    
    Indicates which optional features the client supports.
    """
    elicitation: Optional[str]
    experimental: Optional[List[Tuple[str, str]]]
    list_changed: Optional[ClientLists]
    sampling: Optional[str]

@dataclass
class Implementation:
    """
    MCP implementation metadata
    
    Identifies the server or client implementation.
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#implementation>
    """
    name: str
    title: Optional[str]
    version: str

@dataclass
class Annotations:
    """
    =========================================================================
    Annotations
    =========================================================================
    Annotations that inform how clients use or display objects
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#annotations>
    """
    audience: Optional[List[Role]]
    last_modified: Optional[str]
    priority: Optional[float]

@dataclass
class ContentOptions:
    """
    Options for content blocks
    """
    annotations: Optional[Annotations]
    meta: Optional[str]

@dataclass
class ResourceLinkOptions:
    """
    Options for resource link content
    """
    title: Optional[str]
    description: Optional[str]
    size: Optional[int]
    mime_type: Optional[str]
    annotations: Optional[Annotations]
    meta: Optional[str]

@dataclass
class ResourceLinkContent:
    """
    A resource link included in a prompt or tool call result
    
    Resource links reference resources that the server can read. They may not
    appear in resources/list responses.
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#resourcelink>
    """
    uri: str
    name: str
    options: Optional[ResourceLinkOptions]

@dataclass
class Blob:
    """
    Binary content (image or audio) with MIME type
    
    Used for images and audio provided to or from an LLM.
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#imagecontent>
    """
    data: BlobData
    mime_type: str
    options: Optional[ContentOptions]

@dataclass
class TextContent:
    """
    Text content block
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#textcontent>
    """
    text: TextData
    options: Optional[ContentOptions]

@dataclass
class EmbeddedResourceContent:
    """
    Embedded resource with content options
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#embeddedresource>
    """
    resource: ResourceContents
    options: Optional[ContentOptions]


@dataclass
class ContentBlock_Text:
    value: TextContent


@dataclass
class ContentBlock_Image:
    value: Blob


@dataclass
class ContentBlock_Audio:
    value: Blob


@dataclass
class ContentBlock_ResourceLink:
    value: ResourceLinkContent


@dataclass
class ContentBlock_EmbeddedResource:
    value: EmbeddedResourceContent


ContentBlock = Union[ContentBlock_Text, ContentBlock_Image, ContentBlock_Audio, ContentBlock_ResourceLink, ContentBlock_EmbeddedResource]
"""
Content blocks that can be included in messages

Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#contentblock>
"""


@dataclass
class PromptMessage:
    """
    A message in a prompt with role and content
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#promptmessage>
    """
    content: ContentBlock
    role: Role

@dataclass
class ToolAnnotations:
    """
    Tool-specific annotations
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#toolannotations>
    """
    title: Optional[str]
    read_only_hint: Optional[bool]
    destructive_hint: Optional[bool]
    idempotent_hint: Optional[bool]
    open_world_hint: Optional[bool]

@dataclass
class ToolOptions:
    """
    =========================================================================
    Tool Types
    =========================================================================
    Optional tool properties
    """
    meta: Optional[str]
    annotations: Optional[ToolAnnotations]
    description: Optional[str]
    output_schema: Optional[str]
    title: Optional[str]

@dataclass
class NextCursorOptions:
    """
    Pagination options for list results
    """
    meta: Optional[str]
    next_cursor: Optional[str]

@dataclass
class Tool:
    """
    Tool definition
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#tool>
    """
    name: str
    input_schema: str
    options: Optional[ToolOptions]

@dataclass
class ResourceOptions:
    """
    =========================================================================
    Resource Types
    =========================================================================
    Resource optional properties
    """
    size: Optional[int]
    title: Optional[str]
    description: Optional[str]
    mime_type: Optional[str]
    annotations: Optional[Annotations]
    meta: Optional[str]

@dataclass
class McpResource:
    """
    Resource definition
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#resource>
    """
    uri: str
    name: str
    options: Optional[ResourceOptions]

@dataclass
class MetaOptions:
    """
    Generic metadata options
    """
    meta: Optional[str]

@dataclass
class ResourceTemplateOptions:
    """
    Resource template optional properties
    """
    description: Optional[str]
    title: Optional[str]
    mime_type: Optional[str]
    annotations: Optional[Annotations]
    meta: Optional[str]

@dataclass
class ResourceTemplate:
    """
    Resource template with URI template pattern
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#resourcetemplate>
    """
    uri_template: str
    name: str
    options: Optional[ResourceTemplateOptions]

@dataclass
class PromptArgument:
    """
    =========================================================================
    Prompt Types
    =========================================================================
    Prompt argument definition
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#promptargument>
    """
    name: str
    description: Optional[str]
    required: Optional[bool]
    title: Optional[str]

@dataclass
class PromptOptions:
    """
    Prompt optional properties
    """
    meta: Optional[str]
    arguments: Optional[List[PromptArgument]]
    description: Optional[str]
    title: Optional[str]

@dataclass
class Prompt:
    """
    Prompt definition
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#prompt>
    """
    name: str
    options: Optional[PromptOptions]

@dataclass
class DescriptionOptions:
    """
    Generic description options
    """
    meta: Optional[str]
    description: Optional[str]

class StringSchemaFormat(Enum):
    """
    =========================================================================
    Schema Types (for Elicitation)
    =========================================================================
    String schema format constraints
    """
    URI = 0
    EMAIL = 1
    DATE = 2
    DATE_TIME = 3

@dataclass
class StringSchema:
    """
    JSON Schema for string type
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#stringschema>
    """
    description: Optional[str]
    format: Optional[StringSchemaFormat]
    max_length: Optional[int]
    min_length: Optional[int]
    title: Optional[str]

class NumberSchemaType(Enum):
    """
    Number schema type
    """
    NUMBER = 0
    INTEGER = 1

@dataclass
class NumberSchema:
    """
    JSON Schema for number/integer type
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#numberschema>
    """
    description: Optional[str]
    maximum: Optional[float]
    minimum: Optional[float]
    title: Optional[str]
    type: NumberSchemaType

@dataclass
class BooleanSchema:
    """
    JSON Schema for boolean type
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#booleanschema>
    """
    default: Optional[bool]
    description: Optional[str]
    title: Optional[str]

@dataclass
class EnumSchema:
    """
    JSON Schema for enum type
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#enumschema>
    """
    description: Optional[str]
    enum: List[str]
    enum_names: Optional[List[str]]
    title: Optional[str]


@dataclass
class PrimitiveSchemaDefinition_StringSchema:
    value: StringSchema


@dataclass
class PrimitiveSchemaDefinition_NumberSchema:
    value: NumberSchema


@dataclass
class PrimitiveSchemaDefinition_BooleanSchema:
    value: BooleanSchema


@dataclass
class PrimitiveSchemaDefinition_EnumSchema:
    value: EnumSchema


PrimitiveSchemaDefinition = Union[PrimitiveSchemaDefinition_StringSchema, PrimitiveSchemaDefinition_NumberSchema, PrimitiveSchemaDefinition_BooleanSchema, PrimitiveSchemaDefinition_EnumSchema]
"""
Primitive schema types
"""


@dataclass
class RequestedSchema:
    """
    Schema requested from client during elicitation
    """
    properties: List[Tuple[str, PrimitiveSchemaDefinition]]
    required: Optional[List[str]]

class ElicitResultAction(Enum):
    """
    Elicitation result action
    """
    ACCEPT = 0
    DECLINE = 1
    CANCEL = 2


@dataclass
class ElicitResultContent_String:
    value: str


@dataclass
class ElicitResultContent_Number:
    value: float


@dataclass
class ElicitResultContent_Boolean:
    value: bool


ElicitResultContent = Union[ElicitResultContent_String, ElicitResultContent_Number, ElicitResultContent_Boolean]
"""
Elicitation result content value
"""


@dataclass
class InitializeRequest:
    """
    =========================================================================
    Request/Response Types: Initialize
    =========================================================================
    Initialize request parameters
    
    Sent by client to begin an MCP session.
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#initializerequest>
    """
    capabilities: ClientCapabilities
    client_info: Implementation
    protocol_version: ProtocolVersion

@dataclass
class InitializeResultOptions:
    """
    Initialize result optional properties
    """
    instructions: Optional[str]
    meta: Optional[str]

@dataclass
class InitializeResult:
    """
    Initialize result structure
    
    Returned by server in response to initialize request.
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#initializeresult>
    """
    meta: Optional[str]
    server_info: Implementation
    capabilities: ServerCapabilities
    protocol_version: ProtocolVersion
    options: Optional[InitializeResultOptions]

@dataclass
class ListToolsRequest:
    """
    =========================================================================
    Request/Response Types: Tools
    =========================================================================
    List tools request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listtoolsrequest>
    """
    cursor: Optional[str]

@dataclass
class ListToolsResult:
    """
    List tools result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listtoolsresult>
    """
    meta: Optional[str]
    next_cursor: Optional[str]
    tools: List[Tool]

@dataclass
class CallToolRequest:
    """
    Call tool request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#calltoolrequest>
    """
    name: str
    arguments: Optional[str]

@dataclass
class CallToolResult:
    """
    Call tool result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#calltoolresult>
    """
    meta: Optional[str]
    content: List[ContentBlock]
    is_error: Optional[bool]
    structured_content: Optional[str]

@dataclass
class ListResourcesRequest:
    """
    =========================================================================
    Request/Response Types: Resources
    =========================================================================
    List resources request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listresourcesrequest>
    """
    cursor: Optional[str]

@dataclass
class ListResourcesResult:
    """
    List resources result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listresourcesresult>
    """
    meta: Optional[str]
    next_cursor: Optional[str]
    resources: List[McpResource]

@dataclass
class ReadResourceRequest:
    """
    Read resource request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#readresourcerequest>
    """
    uri: str

@dataclass
class ReadResourceResult:
    """
    Read resource result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#readresourceresult>
    """
    meta: Optional[str]
    contents: List[ResourceContents]

@dataclass
class ListResourceTemplatesRequest:
    """
    List resource templates request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listresourcetemplatesrequest>
    """
    cursor: Optional[str]

@dataclass
class ListResourceTemplatesResult:
    """
    List resource templates result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listresourcetemplatesresult>
    """
    meta: Optional[str]
    next_cursor: Optional[str]
    resource_templates: List[ResourceTemplate]

@dataclass
class ListPromptsRequest:
    """
    =========================================================================
    Request/Response Types: Prompts
    =========================================================================
    List prompts request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listpromptsrequest>
    """
    cursor: Optional[str]

@dataclass
class ListPromptsResult:
    """
    List prompts result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listpromptsresult>
    """
    meta: Optional[str]
    next_cursor: Optional[str]
    prompts: List[Prompt]

@dataclass
class GetPromptRequest:
    """
    Get prompt request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#getpromptrequest>
    """
    name: str
    arguments: Optional[str]

@dataclass
class GetPromptResult:
    """
    Get prompt result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#getpromptresult>
    """
    meta: Optional[str]
    description: Optional[str]
    messages: List[PromptMessage]

@dataclass
class CompletionArgument:
    """
    =========================================================================
    Request/Response Types: Completion
    =========================================================================
    Completion argument
    """
    name: str
    value: str

@dataclass
class CompletionContext:
    """
    Completion context
    """
    arguments: Optional[str]

@dataclass
class CompletionPromptReference:
    """
    Prompt reference for completion
    """
    name: str
    title: Optional[str]


@dataclass
class CompletionReference_Prompt:
    value: CompletionPromptReference


@dataclass
class CompletionReference_ResourceTemplate:
    value: str


CompletionReference = Union[CompletionReference_Prompt, CompletionReference_ResourceTemplate]
"""
Reference types for completion
"""


@dataclass
class CompleteRequest:
    """
    Complete request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#completerequest>
    """
    argument: CompletionArgument
    ref: CompletionReference
    context: Optional[CompletionContext]

@dataclass
class CompleteResult:
    """
    Complete result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#completeresult>
    """
    meta: Optional[str]
    has_more: Optional[bool]
    total: Optional[int]
    values: List[str]

@dataclass
class ElicitRequest:
    """
    =========================================================================
    Request/Response Types: Elicitation
    =========================================================================
    Elicit request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#elicitrequest>
    """
    message: str
    requested_schema: RequestedSchema

@dataclass
class ElicitResult:
    """
    Elicit result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#elicitresult>
    """
    meta: Optional[str]
    action: ElicitResultAction
    content: Optional[List[Tuple[str, ElicitResultContent]]]

@dataclass
class ListRootsRequest:
    """
    =========================================================================
    Request/Response Types: Roots
    =========================================================================
    List roots request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listrootsrequest>
    """
    meta: Optional[str]
    progress_token: Optional[ProgressToken]

@dataclass
class Root:
    """
    Root directory or file
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#root>
    """
    meta: Optional[str]
    name: Optional[str]
    uri: str

@dataclass
class ListRootsResult:
    """
    List roots result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#listrootsresult>
    """
    meta: Optional[str]
    roots: List[Root]

@dataclass
class PingRequest:
    """
    =========================================================================
    Request/Response Types: Ping
    =========================================================================
    Ping request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#pingrequest>
    """
    meta: Optional[str]
    progress_token: Optional[ProgressToken]
    extras: List[Tuple[str, str]]

class IncludeContext(Enum):
    """
    =========================================================================
    Request/Response Types: Sampling
    =========================================================================
    Context inclusion for sampling
    """
    NONE = 0
    THIS_SERVER = 1
    ALL_SERVERS = 2

class SamplingContent(Enum):
    """
    Sampling content type
    """
    TEXT_CONTENT = 0
    IMAGE_CONTENT = 1
    AUDIO_CONTENT = 2

@dataclass
class SamplingMessage:
    """
    Message for sampling request
    """
    content: SamplingContent
    role: Role

@dataclass
class ModelHint:
    """
    Model hint for sampling
    """
    name: Optional[str]
    extra: Optional[str]

@dataclass
class ModelPreferences:
    """
    Model preferences for sampling
    """
    cost_priority: Optional[float]
    hints: Optional[List[ModelHint]]
    intelligence_priority: Optional[float]
    speed_priority: Optional[float]

@dataclass
class SamplingCreateMessageRequest:
    """
    Sampling create message request
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#createmessagerequest>
    """
    include_context: IncludeContext
    max_tokens: int
    messages: List[SamplingMessage]
    metadata: Optional[str]
    model_preferences: Optional[ModelPreferences]
    stop_sequences: Optional[List[str]]
    system_prompt: Optional[str]
    temperature: Optional[float]

@dataclass
class SamplingCreateMessageResult:
    """
    Sampling create message result
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#createmessageresult>
    """
    meta: Optional[str]
    content: SamplingContent
    model: str
    role: Role
    stop_reason: Optional[str]
    extra: Optional[str]


@dataclass
class ClientRequest_Initialize:
    value: InitializeRequest


@dataclass
class ClientRequest_ToolsList:
    value: ListToolsRequest


@dataclass
class ClientRequest_ToolsCall:
    value: CallToolRequest


@dataclass
class ClientRequest_ResourcesList:
    value: ListResourcesRequest


@dataclass
class ClientRequest_ResourcesRead:
    value: ReadResourceRequest


@dataclass
class ClientRequest_ResourcesTemplatesList:
    value: ListResourceTemplatesRequest


@dataclass
class ClientRequest_PromptsList:
    value: ListPromptsRequest


@dataclass
class ClientRequest_PromptsGet:
    value: GetPromptRequest


@dataclass
class ClientRequest_CompletionComplete:
    value: CompleteRequest


@dataclass
class ClientRequest_LoggingSetLevel:
    value: LogLevel


@dataclass
class ClientRequest_Ping:
    value: PingRequest


@dataclass
class ClientRequest_ResourcesSubscribe:
    value: str


@dataclass
class ClientRequest_ResourcesUnsubscribe:
    value: str


ClientRequest = Union[ClientRequest_Initialize, ClientRequest_ToolsList, ClientRequest_ToolsCall, ClientRequest_ResourcesList, ClientRequest_ResourcesRead, ClientRequest_ResourcesTemplatesList, ClientRequest_PromptsList, ClientRequest_PromptsGet, ClientRequest_CompletionComplete, ClientRequest_LoggingSetLevel, ClientRequest_Ping, ClientRequest_ResourcesSubscribe, ClientRequest_ResourcesUnsubscribe]
"""
=========================================================================
Request/Response Variants
=========================================================================
Client requests (sent to server)
"""



@dataclass
class ServerRequest_ElicitationCreate:
    value: ElicitRequest


@dataclass
class ServerRequest_RootsList:
    value: ListRootsRequest


@dataclass
class ServerRequest_SamplingCreateMessage:
    value: SamplingCreateMessageRequest


@dataclass
class ServerRequest_Ping:
    value: PingRequest


ServerRequest = Union[ServerRequest_ElicitationCreate, ServerRequest_RootsList, ServerRequest_SamplingCreateMessage, ServerRequest_Ping]
"""
Server requests (sent to client)
"""



@dataclass
class McpRequest_Server:
    value: ServerRequest


@dataclass
class McpRequest_Client:
    value: ClientRequest


McpRequest = Union[McpRequest_Server, McpRequest_Client]
"""
MCP request (client or server)
"""



@dataclass
class ServerResponse_Initialize:
    value: InitializeResult


@dataclass
class ServerResponse_ToolsList:
    value: ListToolsResult


@dataclass
class ServerResponse_ToolsCall:
    value: CallToolResult


@dataclass
class ServerResponse_ResourcesList:
    value: ListResourcesResult


@dataclass
class ServerResponse_ResourcesRead:
    value: ReadResourceResult


@dataclass
class ServerResponse_ResourcesTemplatesList:
    value: ListResourceTemplatesResult


@dataclass
class ServerResponse_PromptsList:
    value: ListPromptsResult


@dataclass
class ServerResponse_PromptsGet:
    value: GetPromptResult


@dataclass
class ServerResponse_CompletionComplete:
    value: CompleteResult


ServerResponse = Union[ServerResponse_Initialize, ServerResponse_ToolsList, ServerResponse_ToolsCall, ServerResponse_ResourcesList, ServerResponse_ResourcesRead, ServerResponse_ResourcesTemplatesList, ServerResponse_PromptsList, ServerResponse_PromptsGet, ServerResponse_CompletionComplete]
"""
Server responses (to client requests)
"""



@dataclass
class ClientResponse_ElicitationCreate:
    value: ElicitResult


@dataclass
class ClientResponse_RootsList:
    value: ListRootsResult


@dataclass
class ClientResponse_SamplingCreateMessage:
    value: SamplingCreateMessageResult


ClientResponse = Union[ClientResponse_ElicitationCreate, ClientResponse_RootsList, ClientResponse_SamplingCreateMessage]
"""
Client responses (to server requests)
"""



@dataclass
class McpResponse_Server:
    value: ServerResponse


@dataclass
class McpResponse_Client:
    value: ClientResponse


McpResponse = Union[McpResponse_Server, McpResponse_Client]
"""
MCP response (client or server)
"""


@dataclass
class LoggingMessageNotification:
    """
    =========================================================================
    Notification Types
    =========================================================================
    Logging message notification
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#loggingmessagenotification>
    """
    data: str
    level: LogLevel
    logger: Optional[str]

@dataclass
class CancelledNotification:
    """
    Cancelled notification
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#cancellednotification>
    """
    request_id: RequestId
    reason: Optional[str]

@dataclass
class ProgressNotification:
    """
    Progress notification
    
    Spec: <https://spec.modelcontextprotocol.io/specification/2025-06-18/schema#progressnotification>
    """
    progress_token: ProgressToken
    progress: float
    total: Optional[float]
    message: Optional[str]

@dataclass
class CommonNotification:
    """
    Common notification fields
    """
    meta: Optional[str]
    extras: Optional[str]


@dataclass
class ServerNotification_ToolsListChanged:
    value: CommonNotification


@dataclass
class ServerNotification_ResourcesListChanged:
    value: CommonNotification


@dataclass
class ServerNotification_PromptsListChanged:
    value: CommonNotification


@dataclass
class ServerNotification_LoggingMessage:
    value: LoggingMessageNotification


@dataclass
class ServerNotification_Cancelled:
    value: CancelledNotification


@dataclass
class ServerNotification_Progress:
    value: ProgressNotification


ServerNotification = Union[ServerNotification_ToolsListChanged, ServerNotification_ResourcesListChanged, ServerNotification_PromptsListChanged, ServerNotification_LoggingMessage, ServerNotification_Cancelled, ServerNotification_Progress]
"""
Server notifications (sent to client)
"""



@dataclass
class ClientNotification_Initialized:
    value: CommonNotification


@dataclass
class ClientNotification_RootsListChanged:
    value: CommonNotification


@dataclass
class ClientNotification_Cancelled:
    value: CancelledNotification


@dataclass
class ClientNotification_Progress:
    value: ProgressNotification


ClientNotification = Union[ClientNotification_Initialized, ClientNotification_RootsListChanged, ClientNotification_Cancelled, ClientNotification_Progress]
"""
Client notifications (sent to server)
"""



@dataclass
class McpNotification_Server:
    value: ServerNotification


@dataclass
class McpNotification_Client:
    value: ClientNotification


McpNotification = Union[McpNotification_Server, McpNotification_Client]
"""
MCP notification (client or server)
"""


@dataclass
class Error:
    """
    =========================================================================
    Error Types
    =========================================================================
    Error structure
    """
    id: Optional[RequestId]
    code: int
    message: str
    data: Optional[str]


@dataclass
class ErrorCode_ParseError:
    value: Error


@dataclass
class ErrorCode_InvalidRequest:
    value: Error


@dataclass
class ErrorCode_MethodNotFound:
    value: Error


@dataclass
class ErrorCode_InvalidParams:
    value: Error


@dataclass
class ErrorCode_InternalError:
    value: Error


@dataclass
class ErrorCode_Server:
    value: Error


@dataclass
class ErrorCode_JsonRpc:
    value: Error


@dataclass
class ErrorCode_Mcp:
    value: Error


ErrorCode = Union[ErrorCode_ParseError, ErrorCode_InvalidRequest, ErrorCode_MethodNotFound, ErrorCode_InvalidParams, ErrorCode_InternalError, ErrorCode_Server, ErrorCode_JsonRpc, ErrorCode_Mcp]
"""
Standard JSON-RPC error codes
"""


@dataclass
class ClientContext:
    """
    =========================================================================
    Context Types
    =========================================================================
    Client context passed to server handlers
    
    Contains information about the requesting client for authorization
    and personalization.
    """
    identity_claims: Optional[str]
    session_id: Optional[bytes]
    output: Optional[streams.OutputStream]

@dataclass
class ServerContext:
    """
    Server context passed to client handlers
    
    Contains information about the server connection.
    """
    output: Optional[streams.OutputStream]


