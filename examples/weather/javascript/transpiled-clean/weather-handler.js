import { stderr, stdin, stdout, terminalInput, terminalOutput, terminalStderr, terminalStdin, terminalStdout } from '@bytecodealliance/preview2-shim/cli';
import { monotonicClock, wallClock } from '@bytecodealliance/preview2-shim/clocks';
import { preopens, types } from '@bytecodealliance/preview2-shim/filesystem';
import { outgoingHandler, types as types$1 } from '@bytecodealliance/preview2-shim/http';
import { error, poll as poll$1, streams } from '@bytecodealliance/preview2-shim/io';
import { random } from '@bytecodealliance/preview2-shim/random';
const { getStderr } = stderr;
const { getStdin } = stdin;
const { getStdout } = stdout;
const { TerminalInput } = terminalInput;
const { TerminalOutput } = terminalOutput;
const { getTerminalStderr } = terminalStderr;
const { getTerminalStdin } = terminalStdin;
const { getTerminalStdout } = terminalStdout;
const { now,
  resolution,
  subscribeDuration,
  subscribeInstant } = monotonicClock;
const { now: now$1,
  resolution: resolution$1 } = wallClock;
const { getDirectories } = preopens;
const { Descriptor,
  filesystemErrorCode } = types;
const { handle } = outgoingHandler;
const { Fields,
  FutureIncomingResponse,
  IncomingBody,
  IncomingRequest,
  IncomingResponse,
  OutgoingBody,
  OutgoingRequest,
  OutgoingResponse,
  RequestOptions,
  ResponseOutparam } = types$1;
const { Error: Error$1 } = error;
const { Pollable,
  poll } = poll$1;
const { InputStream,
  OutputStream } = streams;
const { getRandomBytes,
  getRandomU64 } = random;

let dv = new DataView(new ArrayBuffer());
const dataView = mem => dv.buffer === mem.buffer ? dv : dv = new DataView(mem.buffer);

const toUint64 = val => BigInt.asUintN(64, BigInt(val));

function toUint16(val) {
  val >>>= 0;
  val %= 2 ** 16;
  return val;
}

function toUint32(val) {
  return val >>> 0;
}

function toUint8(val) {
  val >>>= 0;
  val %= 2 ** 8;
  return val;
}

const utf8Decoder = new TextDecoder();

const utf8Encoder = new TextEncoder();
let utf8EncodedLen = 0;
function utf8Encode(s, realloc, memory) {
  if (typeof s !== 'string') throw new TypeError('expected a string');
  if (s.length === 0) {
    utf8EncodedLen = 0;
    return 1;
  }
  let buf = utf8Encoder.encode(s);
  let ptr = realloc(0, 0, 1, buf.length);
  new Uint8Array(memory.buffer).set(buf, ptr);
  utf8EncodedLen = buf.length;
  return ptr;
}

const T_FLAG = 1 << 30;

function rscTableCreateOwn (table, rep) {
  const free = table[0] & ~T_FLAG;
  if (free === 0) {
    table.push(0);
    table.push(rep | T_FLAG);
    return (table.length >> 1) - 1;
  }
  table[0] = table[free << 1];
  table[free << 1] = 0;
  table[(free << 1) + 1] = rep | T_FLAG;
  return free;
}

function rscTableRemove (table, handle) {
  const scope = table[handle << 1];
  const val = table[(handle << 1) + 1];
  const own = (val & T_FLAG) !== 0;
  const rep = val & ~T_FLAG;
  if (val === 0 || (scope & T_FLAG) !== 0) throw new TypeError('Invalid handle');
  table[handle << 1] = table[0] | T_FLAG;
  table[0] = handle | T_FLAG;
  return { rep, scope, own };
}

let curResourceBorrows = [];

let NEXT_TASK_ID = 0n;
function startCurrentTask(componentIdx, isAsync, entryFnName) {
  _debugLog('[startCurrentTask()] args', { componentIdx, isAsync });
  if (componentIdx === undefined || componentIdx === null) {
    throw new Error('missing/invalid component instance index while starting task');
  }
  const tasks = ASYNC_TASKS_BY_COMPONENT_IDX.get(componentIdx);
  
  const nextId = ++NEXT_TASK_ID;
  const newTask = new AsyncTask({ id: nextId, componentIdx, isAsync, entryFnName });
  const newTaskMeta = { id: nextId, componentIdx, task: newTask };
  
  ASYNC_CURRENT_TASK_IDS.push(nextId);
  ASYNC_CURRENT_COMPONENT_IDXS.push(componentIdx);
  
  if (!tasks) {
    ASYNC_TASKS_BY_COMPONENT_IDX.set(componentIdx, [newTaskMeta]);
    return nextId;
  } else {
    tasks.push(newTaskMeta);
  }
  
  return nextId;
}

function endCurrentTask(componentIdx, taskId) {
  _debugLog('[endCurrentTask()] args', { componentIdx });
  componentIdx ??= ASYNC_CURRENT_COMPONENT_IDXS.at(-1);
  taskId ??= ASYNC_CURRENT_TASK_IDS.at(-1);
  if (componentIdx === undefined || componentIdx === null) {
    throw new Error('missing/invalid component instance index while ending current task');
  }
  const tasks = ASYNC_TASKS_BY_COMPONENT_IDX.get(componentIdx);
  if (!tasks || !Array.isArray(tasks)) {
    throw new Error('missing/invalid tasks for component instance while ending task');
  }
  if (tasks.length == 0) {
    throw new Error('no current task(s) for component instance while ending task');
  }
  
  if (taskId) {
    const last = tasks[tasks.length - 1];
    if (last.id !== taskId) {
      throw new Error('current task does not match expected task ID');
    }
  }
  
  ASYNC_CURRENT_TASK_IDS.pop();
  ASYNC_CURRENT_COMPONENT_IDXS.pop();
  
  return tasks.pop();
}
const ASYNC_TASKS_BY_COMPONENT_IDX = new Map();
const ASYNC_CURRENT_TASK_IDS = [];
const ASYNC_CURRENT_COMPONENT_IDXS = [];

class AsyncTask {
  static State = {
    INITIAL: 'initial',
    CANCELLED: 'cancelled',
    CANCEL_PENDING: 'cancel-pending',
    CANCEL_DELIVERED: 'cancel-delivered',
    RESOLVED: 'resolved',
  }
  
  static BlockResult = {
    CANCELLED: 'block.cancelled',
    NOT_CANCELLED: 'block.not-cancelled',
  }
  
  #id;
  #componentIdx;
  #state;
  #isAsync;
  #onResolve = null;
  #returnedResults = null;
  #entryFnName = null;
  
  cancelled = false;
  requested = false;
  alwaysTaskReturn = false;
  
  returnCalls =  0;
  storage = [0, 0];
  borrowedHandles = {};
  
  awaitableResume = null;
  awaitableCancel = null;
  
  constructor(opts) {
    if (opts?.id === undefined) { throw new TypeError('missing task ID during task creation'); }
    this.#id = opts.id;
    if (opts?.componentIdx === undefined) {
      throw new TypeError('missing component id during task creation');
    }
    this.#componentIdx = opts.componentIdx;
    this.#state = AsyncTask.State.INITIAL;
    this.#isAsync = opts?.isAsync ?? false;
    this.#entryFnName = opts.entryFnName;
    
    this.#onResolve = (results) => {
      this.#returnedResults = results;
    }
  }
  
  taskState() { return this.#state.slice(); }
  id() { return this.#id; }
  componentIdx() { return this.#componentIdx; }
  isAsync() { return this.#isAsync; }
  getEntryFnName() { return this.#entryFnName; }
  
  takeResults() {
    const results = this.#returnedResults;
    this.#returnedResults = null;
    return results;
  }
  
  mayEnter(task) {
    const cstate = getOrCreateAsyncState(this.#componentIdx);
    if (!cstate.backpressure) {
      _debugLog('[AsyncTask#mayEnter()] disallowed due to backpressure', { taskID: this.#id });
      return false;
    }
    if (!cstate.callingSyncImport()) {
      _debugLog('[AsyncTask#mayEnter()] disallowed due to sync import call', { taskID: this.#id });
      return false;
    }
    const callingSyncExportWithSyncPending = cstate.callingSyncExport && !task.isAsync;
    if (!callingSyncExportWithSyncPending) {
      _debugLog('[AsyncTask#mayEnter()] disallowed due to sync export w/ sync pending', { taskID: this.#id });
      return false;
    }
    return true;
  }
  
  async enter() {
    _debugLog('[AsyncTask#enter()] args', { taskID: this.#id });
    
    // TODO: assert scheduler locked
    // TODO: trap if on the stack
    
    const cstate = getOrCreateAsyncState(this.#componentIdx);
    
    let mayNotEnter = !this.mayEnter(this);
    const componentHasPendingTasks = cstate.pendingTasks > 0;
    if (mayNotEnter || componentHasPendingTasks) {
      
      throw new Error('in enter()'); // TODO: remove
      cstate.pendingTasks.set(this.#id, new Awaitable(new Promise()));
      
      const blockResult = await this.onBlock(awaitable);
      if (blockResult) {
        // TODO: find this pending task in the component
        const pendingTask = cstate.pendingTasks.get(this.#id);
        if (!pendingTask) {
          throw new Error('pending task [' + this.#id + '] not found for component instance');
        }
        cstate.pendingTasks.remove(this.#id);
        this.#onResolve([]);
        return false;
      }
      
      mayNotEnter = !this.mayEnter(this);
      if (!mayNotEnter || !cstate.startPendingTask) {
        throw new Error('invalid component entrance/pending task resolution');
      }
      cstate.startPendingTask = false;
    }
    
    if (!this.isAsync) { cstate.callingSyncExport = true; }
    
    return true;
  }
  
  async waitForEvent(opts) {
    const { waitableSetRep, isAsync } = opts;
    _debugLog('[AsyncTask#waitForEvent()] args', { taskID: this.#id, waitableSetRep, isAsync });
    
    if (this.#isAsync !== isAsync) {
      throw new Error('async waitForEvent called on non-async task');
    }
    
    if (this.status === AsyncTask.State.CANCEL_PENDING) {
      this.#state = AsyncTask.State.CANCEL_DELIVERED;
      return {
        code: ASYNC_EVENT_CODE.TASK_CANCELLED,
        something: 0,
        something: 0,
      };
    }
    
    const state = getOrCreateAsyncState(this.#componentIdx);
    const waitableSet = state.waitableSets.get(waitableSetRep);
    if (!waitableSet) { throw new Error('missing/invalid waitable set'); }
    
    waitableSet.numWaiting += 1;
    let event = null;
    
    while (event == null) {
      const awaitable = new Awaitable(waitableSet.getPendingEvent());
      const waited = await this.blockOn({ awaitable, isAsync, isCancellable: true });
      if (waited) {
        if (this.#state !== AsyncTask.State.INITIAL) {
          throw new Error('task should be in initial state found [' + this.#state + ']');
        }
        this.#state = AsyncTask.State.CANCELLED;
        return {
          code: ASYNC_EVENT_CODE.TASK_CANCELLED,
          something: 0,
          something: 0,
        };
      }
      
      event = waitableSet.poll();
    }
    
    waitableSet.numWaiting -= 1;
    return event;
  }
  
  waitForEventSync(opts) {
    throw new Error('AsyncTask#yieldSync() not implemented')
  }
  
  async pollForEvent(opts) {
    const { waitableSetRep, isAsync } = opts;
    _debugLog('[AsyncTask#pollForEvent()] args', { taskID: this.#id, waitableSetRep, isAsync });
    
    if (this.#isAsync !== isAsync) {
      throw new Error('async pollForEvent called on non-async task');
    }
    
    throw new Error('AsyncTask#pollForEvent() not implemented');
  }
  
  pollForEventSync(opts) {
    throw new Error('AsyncTask#yieldSync() not implemented')
  }
  
  async blockOn(opts) {
    const { awaitable, isCancellable, forCallback } = opts;
    _debugLog('[AsyncTask#blockOn()] args', { taskID: this.#id, awaitable, isCancellable, forCallback });
    
    if (awaitable.resolved() && !ASYNC_DETERMINISM && _coinFlip()) {
      return AsyncTask.BlockResult.NOT_CANCELLED;
    }
    
    const cstate = getOrCreateAsyncState(this.#componentIdx);
    if (forCallback) { cstate.exclusiveRelease(); }
    
    let cancelled = await this.onBlock(awaitable);
    if (cancelled === AsyncTask.BlockResult.CANCELLED && !isCancellable) {
      const secondCancel = await this.onBlock(awaitable);
      if (secondCancel !== AsyncTask.BlockResult.NOT_CANCELLED) {
        throw new Error('uncancellable task was canceled despite second onBlock()');
      }
    }
    
    if (forCallback) {
      const acquired = new Awaitable(cstate.exclusiveLock());
      cancelled = await this.onBlock(acquired);
      if (cancelled === AsyncTask.BlockResult.CANCELLED) {
        const secondCancel = await this.onBlock(acquired);
        if (secondCancel !== AsyncTask.BlockResult.NOT_CANCELLED) {
          throw new Error('uncancellable callback task was canceled despite second onBlock()');
        }
      }
    }
    
    if (cancelled === AsyncTask.BlockResult.CANCELLED) {
      if (this.#state !== AsyncTask.State.INITIAL) {
        throw new Error('cancelled task is not at initial state');
      }
      if (isCancellable) {
        this.#state = AsyncTask.State.CANCELLED;
        return AsyncTask.BlockResult.CANCELLED;
      } else {
        this.#state = AsyncTask.State.CANCEL_PENDING;
        return AsyncTask.BlockResult.NOT_CANCELLED;
      }
    }
    
    return AsyncTask.BlockResult.NOT_CANCELLED;
  }
  
  async onBlock(awaitable) {
    _debugLog('[AsyncTask#onBlock()] args', { taskID: this.#id, awaitable });
    if (!(awaitable instanceof Awaitable)) {
      throw new Error('invalid awaitable during onBlock');
    }
    
    // Build a promise that this task can await on which resolves when it is awoken
    const { promise, resolve, reject } = Promise.withResolvers();
    this.awaitableResume = () => {
      _debugLog('[AsyncTask] resuming after onBlock', { taskID: this.#id });
      resolve();
    };
    this.awaitableCancel = (err) => {
      _debugLog('[AsyncTask] rejecting after onBlock', { taskID: this.#id, err });
      reject(err);
    };
    
    // Park this task/execution to be handled later
    const state = getOrCreateAsyncState(this.#componentIdx);
    state.parkTaskOnAwaitable({ awaitable, task: this });
    
    try {
      await promise;
      return AsyncTask.BlockResult.NOT_CANCELLED;
    } catch (err) {
      // rejection means task cancellation
      return AsyncTask.BlockResult.CANCELLED;
    }
  }
  
  // NOTE: this should likely be moved to a SubTask class
  async asyncOnBlock(awaitable) {
    _debugLog('[AsyncTask#asyncOnBlock()] args', { taskID: this.#id, awaitable });
    if (!(awaitable instanceof Awaitable)) {
      throw new Error('invalid awaitable during onBlock');
    }
    // TODO: watch for waitable AND cancellation
    // TODO: if it WAS cancelled:
    // - return true
    // - only once per subtask
    // - do not wait on the scheduler
    // - control flow should go to the subtask (only once)
    // - Once subtask blocks/resolves, reqlinquishControl() will tehn resolve request_cancel_end (without scheduler lock release)
    // - control flow goes back to request_cancel
    //
    // Subtask cancellation should work similarly to an async import call -- runs sync up until
    // the subtask blocks or resolves
    //
    throw new Error('AsyncTask#asyncOnBlock() not yet implemented');
  }
  
  async yield(opts) {
    const { isCancellable, forCallback } = opts;
    _debugLog('[AsyncTask#yield()] args', { taskID: this.#id, isCancellable, forCallback });
    
    if (isCancellable && this.status === AsyncTask.State.CANCEL_PENDING) {
      this.#state = AsyncTask.State.CANCELLED;
      return {
        code: ASYNC_EVENT_CODE.TASK_CANCELLED,
        payload: [0, 0],
      };
    }
    
    // TODO: Awaitables need to *always* trigger the parking mechanism when they're done...?
    // TODO: Component async state should remember which awaitables are done and work to clear tasks waiting
    
    const blockResult = await this.blockOn({
      awaitable: new Awaitable(new Promise(resolve => setTimeout(resolve, 0))),
      isCancellable,
      forCallback,
    });
    
    if (blockResult === AsyncTask.BlockResult.CANCELLED) {
      if (this.#state !== AsyncTask.State.INITIAL) {
        throw new Error('task should be in initial state found [' + this.#state + ']');
      }
      this.#state = AsyncTask.State.CANCELLED;
      return {
        code: ASYNC_EVENT_CODE.TASK_CANCELLED,
        payload: [0, 0],
      };
    }
    
    return {
      code: ASYNC_EVENT_CODE.NONE,
      payload: [0, 0],
    };
  }
  
  yieldSync(opts) {
    throw new Error('AsyncTask#yieldSync() not implemented')
  }
  
  cancel() {
    _debugLog('[AsyncTask#cancel()] args', { });
    if (!this.taskState() !== AsyncTask.State.CANCEL_DELIVERED) {
      throw new Error('invalid task state for cancellation');
    }
    if (this.borrowedHandles.length > 0) { throw new Error('task still has borrow handles'); }
    
    this.#onResolve([]);
    this.#state = AsyncTask.State.RESOLVED;
  }
  
  resolve(result) {
    if (this.#state === AsyncTask.State.RESOLVED) {
      throw new Error('task is already resolved');
    }
    if (this.borrowedHandles.length > 0) { throw new Error('task still has borrow handles'); }
    this.#onResolve(result);
    this.#state = AsyncTask.State.RESOLVED;
  }
  
  exit() {
    // TODO: ensure there is only one task at a time (scheduler.lock() functionality)
    if (this.#state !== AsyncTask.State.RESOLVED) {
      throw new Error('task exited without resolution');
    }
    if (this.borrowedHandles > 0) {
      throw new Error('task exited without clearing borrowed handles');
    }
    
    const state = getOrCreateAsyncState(this.#componentIdx);
    if (!state) { throw new Error('missing async state for component [' + this.#componentIdx + ']'); }
    if (!this.#isAsync && !state.inSyncExportCall) {
      throw new Error('sync task must be run from components known to be in a sync export call');
    }
    state.inSyncExportCall = false;
    
    this.startPendingTask();
  }
  
  startPendingTask(opts) {
    // TODO: implement
  }
  
}

function unpackCallbackResult(result) {
  _debugLog('[unpackCallbackResult()] args', { result });
  if (!(_typeCheckValidI32(result))) { throw new Error('invalid callback return value [' + result + '], not a valid i32'); }
  const eventCode = result & 0xF;
  if (eventCode < 0 || eventCode > 3) {
    throw new Error('invalid async return value [' + eventCode + '], outside callback code range');
  }
  if (result < 0 || result >= 2**32) { throw new Error('invalid callback result'); }
  // TODO: table max length check?
  const waitableSetIdx = result >> 4;
  return [eventCode, waitableSetIdx];
}
const ASYNC_STATE = new Map();

function getOrCreateAsyncState(componentIdx, init) {
  if (!ASYNC_STATE.has(componentIdx)) {
    ASYNC_STATE.set(componentIdx, new ComponentAsyncState());
  }
  return ASYNC_STATE.get(componentIdx);
}

class ComponentAsyncState {
  #callingAsyncImport = false;
  #syncImportWait = Promise.withResolvers();
  #lock = null;
  
  mayLeave = false;
  waitableSets = new RepTable();
  waitables = new RepTable();
  
  #parkedTasks = new Map();
  
  callingSyncImport(val) {
    if (val === undefined) { return this.#callingAsyncImport; }
    if (typeof val !== 'boolean') { throw new TypeError('invalid setting for async import'); }
    const prev = this.#callingAsyncImport;
    this.#callingAsyncImport = val;
    if (prev === true && this.#callingAsyncImport === false) {
      this.#notifySyncImportEnd();
    }
  }
  
  #notifySyncImportEnd() {
    const existing = this.#syncImportWait;
    this.#syncImportWait = Promise.withResolvers();
    existing.resolve();
  }
  
  async waitForSyncImportCallEnd() {
    await this.#syncImportWait.promise;
  }
  
  parkTaskOnAwaitable(args) {
    if (!args.awaitable) { throw new TypeError('missing awaitable when trying to park'); }
    if (!args.task) { throw new TypeError('missing task when trying to park'); }
    const { awaitable, task } = args;
    
    let taskList = this.#parkedTasks.get(awaitable.id());
    if (!taskList) {
      taskList = [];
      this.#parkedTasks.set(awaitable.id(), taskList);
    }
    taskList.push(task);
    
    this.wakeNextTaskForAwaitable(awaitable);
  }
  
  wakeNextTaskForAwaitable(awaitable) {
    if (!awaitable) { throw new TypeError('missing awaitable when waking next task'); }
    const awaitableID = awaitable.id();
    
    const taskList = this.#parkedTasks.get(awaitableID);
    if (!taskList || taskList.length === 0) {
      _debugLog('[ComponentAsyncState] no tasks waiting for awaitable', { awaitableID: awaitable.id() });
      return;
    }
    
    let task = taskList.shift(); // todo(perf)
    if (!task) { throw new Error('no task in parked list despite previous check'); }
    
    if (!task.awaitableResume) {
      throw new Error('task ready due to awaitable is missing resume', { taskID: task.id(), awaitableID });
    }
    task.awaitableResume();
  }
  
  async exclusiveLock() {  // TODO: use atomics
  if (this.#lock === null) {
    this.#lock = { ticket: 0n };
  }
  
  // Take a ticket for the next valid usage
  const ticket = ++this.#lock.ticket;
  
  _debugLog('[ComponentAsyncState#exclusiveLock()] locking', {
    currentTicket: ticket - 1n,
    ticket
  });
  
  // If there is an active promise, then wait for it
  let finishedTicket;
  while (this.#lock.promise) {
    finishedTicket = await this.#lock.promise;
    if (finishedTicket === ticket - 1n) { break; }
  }
  
  const { promise, resolve } = Promise.withResolvers();
  this.#lock = {
    ticket,
    promise,
    resolve,
  };
  
  return this.#lock.promise;
}

exclusiveRelease() {
  _debugLog('[ComponentAsyncState#exclusiveRelease()] releasing', {
    currentTicket: this.#lock === null ? 'none' : this.#lock.ticket,
  });
  
  if (this.#lock === null) { return; }
  
  const existingLock = this.#lock;
  this.#lock = null;
  existingLock.resolve(existingLock.ticket);
}

isExclusivelyLocked() { return this.#lock !== null; }

}

if (!Promise.withResolvers) {
  Promise.withResolvers = () => {
    let resolve;
    let reject;
    const promise = new Promise((res, rej) => {
      resolve = res;
      reject = rej;
    });
    return { promise, resolve, reject };
  };
}

const _debugLog = (...args) => {
  if (!globalThis?.process?.env?.JCO_DEBUG) { return; }
  console.debug(...args);
}
const ASYNC_DETERMINISM = 'random';
const _coinFlip = () => { return Math.random() > 0.5; };
const I32_MAX = 2_147_483_647;
const I32_MIN = -2_147_483_648;
const _typeCheckValidI32 = (n) => typeof n === 'number' && n >= I32_MIN && n <= I32_MAX;

const base64Compile = str => WebAssembly.compile(typeof Buffer !== 'undefined' ? Buffer.from(str, 'base64') : Uint8Array.from(atob(str), b => b.charCodeAt(0)));

function clampGuest(i, min, max) {
  if (i < min || i > max) throw new TypeError(`must be between ${min} and ${max}`);
  return i;
}

const isNode = typeof process !== 'undefined' && process.versions && process.versions.node;
let _fs;
async function fetchCompile (url) {
  if (isNode) {
    _fs = _fs || await import('node:fs/promises');
    return WebAssembly.compile(await _fs.readFile(url));
  }
  return fetch(url).then(WebAssembly.compileStreaming);
}

const symbolCabiDispose = Symbol.for('cabiDispose');

const symbolRscHandle = Symbol('handle');

const symbolRscRep = Symbol.for('cabiRep');

const symbolDispose = Symbol.dispose || Symbol.for('dispose');

const handleTables = [];

class ComponentError extends Error {
  constructor (value) {
    const enumerable = typeof value !== 'string';
    super(enumerable ? `${String(value)} (see error.payload)` : value);
    Object.defineProperty(this, 'payload', { value, enumerable });
  }
}

function getErrorPayload(e) {
  if (e && hasOwnProperty.call(e, 'payload')) return e.payload;
  if (e instanceof Error) throw e;
  return e;
}

class RepTable {
  #data = [0, null];
  
  insert(val) {
    _debugLog('[RepTable#insert()] args', { val });
    const freeIdx = this.#data[0];
    if (freeIdx === 0) {
      this.#data.push(val);
      this.#data.push(null);
      return (this.#data.length >> 1) - 1;
    }
    this.#data[0] = this.#data[freeIdx];
    const newFreeIdx = freeIdx << 1;
    this.#data[newFreeIdx] = val;
    this.#data[newFreeIdx + 1] = null;
    return free;
  }
  
  get(rep) {
    _debugLog('[RepTable#insert()] args', { rep });
    const baseIdx = idx << 1;
    const val = this.#data[baseIdx];
    return val;
  }
  
  contains(rep) {
    _debugLog('[RepTable#insert()] args', { rep });
    const baseIdx = idx << 1;
    return !!this.#data[baseIdx];
  }
  
  remove(rep) {
    _debugLog('[RepTable#insert()] args', { idx });
    if (this.#data.length === 2) { throw new Error('invalid'); }
    
    const baseIdx = idx << 1;
    const val = this.#data[baseIdx];
    if (val === 0) { throw new Error('invalid resource rep (cannot be 0)'); }
    this.#data[baseIdx] = this.#data[0];
    this.#data[0] = idx;
    return val;
  }
  
  clear() {
    this.#data = [0, null];
  }
}

function throwInvalidBool() {
  throw new TypeError('invalid variant discriminant for bool');
}

const hasOwnProperty = Object.prototype.hasOwnProperty;

const instantiateCore = WebAssembly.instantiate;


let exports0;
const handleTable1 = [T_FLAG, 0];
const captureTable1= new Map();
let captureCnt1 = 0;
handleTables[1] = handleTable1;

function trampoline3(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable1[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable1.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Pollable.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/poll@0.2.3", function="[method]pollable.block"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]pollable.block');
  rsc0.block();
  _debugLog('[iface="wasi:io/poll@0.2.3", function="[method]pollable.block"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  _debugLog('[iface="wasi:io/poll@0.2.3", function="[method]pollable.block"][Instruction::Return]', {
    funcName: '[method]pollable.block',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable2 = [T_FLAG, 0];
const captureTable2= new Map();
let captureCnt2 = 0;
handleTables[2] = handleTable2;

function trampoline4(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable2[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable2.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(InputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.subscribe"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]input-stream.subscribe');
  const ret = rsc0.subscribe();
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.subscribe"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Pollable)) {
    throw new TypeError('Resource error: Not a valid "Pollable" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt1;
    captureTable1.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable1, rep);
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.subscribe"][Instruction::Return]', {
    funcName: '[method]input-stream.subscribe',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

const handleTable3 = [T_FLAG, 0];
const captureTable3= new Map();
let captureCnt3 = 0;
handleTables[3] = handleTable3;

function trampoline5(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable3[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable3.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.subscribe"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]output-stream.subscribe');
  const ret = rsc0.subscribe();
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.subscribe"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Pollable)) {
    throw new TypeError('Resource error: Not a valid "Pollable" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt1;
    captureTable1.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable1, rep);
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.subscribe"][Instruction::Return]', {
    funcName: '[method]output-stream.subscribe',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}


function trampoline6() {
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="now"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'now');
  const ret = now();
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="now"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="now"][Instruction::Return]', {
    funcName: 'now',
    paramCount: 1,
    postReturn: false
  });
  return toUint64(ret);
}


function trampoline7(arg0) {
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="subscribe-instant"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'subscribe-instant');
  const ret = subscribeInstant(BigInt.asUintN(64, arg0));
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="subscribe-instant"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof Pollable)) {
    throw new TypeError('Resource error: Not a valid "Pollable" resource.');
  }
  var handle0 = ret[symbolRscHandle];
  if (!handle0) {
    const rep = ret[symbolRscRep] || ++captureCnt1;
    captureTable1.set(rep, ret);
    handle0 = rscTableCreateOwn(handleTable1, rep);
  }
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="subscribe-instant"][Instruction::Return]', {
    funcName: 'subscribe-instant',
    paramCount: 1,
    postReturn: false
  });
  return handle0;
}


function trampoline8(arg0) {
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="subscribe-duration"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'subscribe-duration');
  const ret = subscribeDuration(BigInt.asUintN(64, arg0));
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="subscribe-duration"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof Pollable)) {
    throw new TypeError('Resource error: Not a valid "Pollable" resource.');
  }
  var handle0 = ret[symbolRscHandle];
  if (!handle0) {
    const rep = ret[symbolRscRep] || ++captureCnt1;
    captureTable1.set(rep, ret);
    handle0 = rscTableCreateOwn(handleTable1, rep);
  }
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="subscribe-duration"][Instruction::Return]', {
    funcName: 'subscribe-duration',
    paramCount: 1,
    postReturn: false
  });
  return handle0;
}


function trampoline9() {
  _debugLog('[iface="wasi:random/random@0.2.3", function="get-random-u64"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-random-u64');
  const ret = getRandomU64();
  _debugLog('[iface="wasi:random/random@0.2.3", function="get-random-u64"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  _debugLog('[iface="wasi:random/random@0.2.3", function="get-random-u64"][Instruction::Return]', {
    funcName: 'get-random-u64',
    paramCount: 1,
    postReturn: false
  });
  return toUint64(ret);
}

const handleTable7 = [T_FLAG, 0];
const captureTable7= new Map();
let captureCnt7 = 0;
handleTables[7] = handleTable7;

function trampoline10() {
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]fields"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[constructor]fields');
  const ret = new Fields();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]fields"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof Fields)) {
    throw new TypeError('Resource error: Not a valid "Fields" resource.');
  }
  var handle0 = ret[symbolRscHandle];
  if (!handle0) {
    const rep = ret[symbolRscRep] || ++captureCnt7;
    captureTable7.set(rep, ret);
    handle0 = rscTableCreateOwn(handleTable7, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]fields"][Instruction::Return]', {
    funcName: '[constructor]fields',
    paramCount: 1,
    postReturn: false
  });
  return handle0;
}


function trampoline11(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.clone"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.clone');
  const ret = rsc0.clone();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.clone"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Fields)) {
    throw new TypeError('Resource error: Not a valid "Fields" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt7;
    captureTable7.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable7, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.clone"][Instruction::Return]', {
    funcName: '[method]fields.clone',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

const handleTable8 = [T_FLAG, 0];
const captureTable8= new Map();
let captureCnt8 = 0;
handleTables[8] = handleTable8;

function trampoline12(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable8[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable8.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.headers"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-request.headers');
  const ret = rsc0.headers();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.headers"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Fields)) {
    throw new TypeError('Resource error: Not a valid "Headers" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt7;
    captureTable7.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable7, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.headers"][Instruction::Return]', {
    funcName: '[method]incoming-request.headers',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

const handleTable10 = [T_FLAG, 0];
const captureTable10= new Map();
let captureCnt10 = 0;
handleTables[10] = handleTable10;

function trampoline13(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  else {
    captureTable7.delete(rep2);
  }
  rscTableRemove(handleTable7, handle1);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]outgoing-request"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[constructor]outgoing-request');
  const ret = new OutgoingRequest(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]outgoing-request"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof OutgoingRequest)) {
    throw new TypeError('Resource error: Not a valid "OutgoingRequest" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt10;
    captureTable10.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable10, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]outgoing-request"][Instruction::Return]', {
    funcName: '[constructor]outgoing-request',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}


function trampoline14(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.headers"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-request.headers');
  const ret = rsc0.headers();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.headers"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Fields)) {
    throw new TypeError('Resource error: Not a valid "Headers" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt7;
    captureTable7.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable7, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.headers"][Instruction::Return]', {
    funcName: '[method]outgoing-request.headers',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

const handleTable14 = [T_FLAG, 0];
const captureTable14= new Map();
let captureCnt14 = 0;
handleTables[14] = handleTable14;

function trampoline15(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable14[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable14.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.status"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-response.status');
  const ret = rsc0.status();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.status"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.status"][Instruction::Return]', {
    funcName: '[method]incoming-response.status',
    paramCount: 1,
    postReturn: false
  });
  return toUint16(ret);
}


function trampoline16(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable14[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable14.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.headers"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-response.headers');
  const ret = rsc0.headers();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.headers"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Fields)) {
    throw new TypeError('Resource error: Not a valid "Headers" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt7;
    captureTable7.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable7, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.headers"][Instruction::Return]', {
    funcName: '[method]incoming-response.headers',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

const handleTable13 = [T_FLAG, 0];
const captureTable13= new Map();
let captureCnt13 = 0;
handleTables[13] = handleTable13;

function trampoline17(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  else {
    captureTable7.delete(rep2);
  }
  rscTableRemove(handleTable7, handle1);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]outgoing-response"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[constructor]outgoing-response');
  const ret = new OutgoingResponse(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]outgoing-response"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof OutgoingResponse)) {
    throw new TypeError('Resource error: Not a valid "OutgoingResponse" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt13;
    captureTable13.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable13, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[constructor]outgoing-response"][Instruction::Return]', {
    funcName: '[constructor]outgoing-response',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}


function trampoline18(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable13[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable13.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.set-status-code"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-response.set-status-code');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.setStatusCode(clampGuest(arg1, 0, 65535))};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.set-status-code"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant3 = ret;
  let variant3_0;
  switch (variant3.tag) {
    case 'ok': {
      const e = variant3.val;
      variant3_0 = 0;
      break;
    }
    case 'err': {
      const e = variant3.val;
      variant3_0 = 1;
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.set-status-code"][Instruction::Return]', {
    funcName: '[method]outgoing-response.set-status-code',
    paramCount: 1,
    postReturn: false
  });
  return variant3_0;
}


function trampoline19(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable13[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable13.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.headers"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-response.headers');
  const ret = rsc0.headers();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.headers"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Fields)) {
    throw new TypeError('Resource error: Not a valid "Headers" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt7;
    captureTable7.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable7, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.headers"][Instruction::Return]', {
    funcName: '[method]outgoing-response.headers',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

const handleTable15 = [T_FLAG, 0];
const captureTable15= new Map();
let captureCnt15 = 0;
handleTables[15] = handleTable15;

function trampoline20(arg0) {
  var handle1 = arg0;
  var rep2 = handleTable15[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable15.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(FutureIncomingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]future-incoming-response.subscribe"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]future-incoming-response.subscribe');
  const ret = rsc0.subscribe();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]future-incoming-response.subscribe"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  if (!(ret instanceof Pollable)) {
    throw new TypeError('Resource error: Not a valid "Pollable" resource.');
  }
  var handle3 = ret[symbolRscHandle];
  if (!handle3) {
    const rep = ret[symbolRscRep] || ++captureCnt1;
    captureTable1.set(rep, ret);
    handle3 = rscTableCreateOwn(handleTable1, rep);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]future-incoming-response.subscribe"][Instruction::Return]', {
    funcName: '[method]future-incoming-response.subscribe',
    paramCount: 1,
    postReturn: false
  });
  return handle3;
}

let exports1;

function trampoline21() {
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="resolution"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'resolution');
  const ret = resolution();
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="resolution"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  _debugLog('[iface="wasi:clocks/monotonic-clock@0.2.3", function="resolution"][Instruction::Return]', {
    funcName: 'resolution',
    paramCount: 1,
    postReturn: false
  });
  return toUint64(ret);
}


function trampoline24() {
  _debugLog('[iface="wasi:cli/stderr@0.2.3", function="get-stderr"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-stderr');
  const ret = getStderr();
  _debugLog('[iface="wasi:cli/stderr@0.2.3", function="get-stderr"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof OutputStream)) {
    throw new TypeError('Resource error: Not a valid "OutputStream" resource.');
  }
  var handle0 = ret[symbolRscHandle];
  if (!handle0) {
    const rep = ret[symbolRscRep] || ++captureCnt3;
    captureTable3.set(rep, ret);
    handle0 = rscTableCreateOwn(handleTable3, rep);
  }
  _debugLog('[iface="wasi:cli/stderr@0.2.3", function="get-stderr"][Instruction::Return]', {
    funcName: 'get-stderr',
    paramCount: 1,
    postReturn: false
  });
  return handle0;
}


function trampoline27() {
  _debugLog('[iface="wasi:cli/stdin@0.2.3", function="get-stdin"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-stdin');
  const ret = getStdin();
  _debugLog('[iface="wasi:cli/stdin@0.2.3", function="get-stdin"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof InputStream)) {
    throw new TypeError('Resource error: Not a valid "InputStream" resource.');
  }
  var handle0 = ret[symbolRscHandle];
  if (!handle0) {
    const rep = ret[symbolRscRep] || ++captureCnt2;
    captureTable2.set(rep, ret);
    handle0 = rscTableCreateOwn(handleTable2, rep);
  }
  _debugLog('[iface="wasi:cli/stdin@0.2.3", function="get-stdin"][Instruction::Return]', {
    funcName: 'get-stdin',
    paramCount: 1,
    postReturn: false
  });
  return handle0;
}


function trampoline28() {
  _debugLog('[iface="wasi:cli/stdout@0.2.3", function="get-stdout"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-stdout');
  const ret = getStdout();
  _debugLog('[iface="wasi:cli/stdout@0.2.3", function="get-stdout"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  if (!(ret instanceof OutputStream)) {
    throw new TypeError('Resource error: Not a valid "OutputStream" resource.');
  }
  var handle0 = ret[symbolRscHandle];
  if (!handle0) {
    const rep = ret[symbolRscRep] || ++captureCnt3;
    captureTable3.set(rep, ret);
    handle0 = rscTableCreateOwn(handleTable3, rep);
  }
  _debugLog('[iface="wasi:cli/stdout@0.2.3", function="get-stdout"][Instruction::Return]', {
    funcName: 'get-stdout',
    paramCount: 1,
    postReturn: false
  });
  return handle0;
}

let exports2;
let memory0;
let realloc0;
let realloc1;

function trampoline29(arg0, arg1, arg2) {
  var len3 = arg1;
  var base3 = arg0;
  var result3 = [];
  for (let i = 0; i < len3; i++) {
    const base = base3 + i * 4;
    var handle1 = dataView(memory0).getInt32(base + 0, true);
    var rep2 = handleTable1[(handle1 << 1) + 1] & ~T_FLAG;
    var rsc0 = captureTable1.get(rep2);
    if (!rsc0) {
      rsc0 = Object.create(Pollable.prototype);
      Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
      Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
    }
    curResourceBorrows.push(rsc0);
    result3.push(rsc0);
  }
  _debugLog('[iface="wasi:io/poll@0.2.3", function="poll"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'poll');
  const ret = poll(result3);
  _debugLog('[iface="wasi:io/poll@0.2.3", function="poll"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var val4 = ret;
  var len4 = val4.length;
  var ptr4 = realloc0(0, 0, 4, len4 * 4);
  var src4 = new Uint8Array(val4.buffer, val4.byteOffset, len4 * 4);
  (new Uint8Array(memory0.buffer, ptr4, len4 * 4)).set(src4);
  dataView(memory0).setUint32(arg2 + 4, len4, true);
  dataView(memory0).setUint32(arg2 + 0, ptr4, true);
  _debugLog('[iface="wasi:io/poll@0.2.3", function="poll"][Instruction::Return]', {
    funcName: 'poll',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable0 = [T_FLAG, 0];
const captureTable0= new Map();
let captureCnt0 = 0;
handleTables[0] = handleTable0;

function trampoline30(arg0, arg1, arg2) {
  var handle1 = arg0;
  var rep2 = handleTable2[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable2.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(InputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.read"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]input-stream.read');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.read(BigInt.asUintN(64, arg1))};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.read"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant6 = ret;
  switch (variant6.tag) {
    case 'ok': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg2 + 0, 0, true);
      var val3 = e;
      var len3 = val3.byteLength;
      var ptr3 = realloc0(0, 0, 1, len3 * 1);
      var src3 = new Uint8Array(val3.buffer || val3, val3.byteOffset, len3 * 1);
      (new Uint8Array(memory0.buffer, ptr3, len3 * 1)).set(src3);
      dataView(memory0).setUint32(arg2 + 8, len3, true);
      dataView(memory0).setUint32(arg2 + 4, ptr3, true);
      break;
    }
    case 'err': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg2 + 0, 1, true);
      var variant5 = e;
      switch (variant5.tag) {
        case 'last-operation-failed': {
          const e = variant5.val;
          dataView(memory0).setInt8(arg2 + 4, 0, true);
          if (!(e instanceof Error$1)) {
            throw new TypeError('Resource error: Not a valid "Error" resource.');
          }
          var handle4 = e[symbolRscHandle];
          if (!handle4) {
            const rep = e[symbolRscRep] || ++captureCnt0;
            captureTable0.set(rep, e);
            handle4 = rscTableCreateOwn(handleTable0, rep);
          }
          dataView(memory0).setInt32(arg2 + 8, handle4, true);
          break;
        }
        case 'closed': {
          dataView(memory0).setInt8(arg2 + 4, 1, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant5.tag)}\` (received \`${variant5}\`) specified for \`StreamError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.read"][Instruction::Return]', {
    funcName: '[method]input-stream.read',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline31(arg0, arg1, arg2) {
  var handle1 = arg0;
  var rep2 = handleTable2[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable2.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(InputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.blocking-read"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]input-stream.blocking-read');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.blockingRead(BigInt.asUintN(64, arg1))};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.blocking-read"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant6 = ret;
  switch (variant6.tag) {
    case 'ok': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg2 + 0, 0, true);
      var val3 = e;
      var len3 = val3.byteLength;
      var ptr3 = realloc0(0, 0, 1, len3 * 1);
      var src3 = new Uint8Array(val3.buffer || val3, val3.byteOffset, len3 * 1);
      (new Uint8Array(memory0.buffer, ptr3, len3 * 1)).set(src3);
      dataView(memory0).setUint32(arg2 + 8, len3, true);
      dataView(memory0).setUint32(arg2 + 4, ptr3, true);
      break;
    }
    case 'err': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg2 + 0, 1, true);
      var variant5 = e;
      switch (variant5.tag) {
        case 'last-operation-failed': {
          const e = variant5.val;
          dataView(memory0).setInt8(arg2 + 4, 0, true);
          if (!(e instanceof Error$1)) {
            throw new TypeError('Resource error: Not a valid "Error" resource.');
          }
          var handle4 = e[symbolRscHandle];
          if (!handle4) {
            const rep = e[symbolRscRep] || ++captureCnt0;
            captureTable0.set(rep, e);
            handle4 = rscTableCreateOwn(handleTable0, rep);
          }
          dataView(memory0).setInt32(arg2 + 8, handle4, true);
          break;
        }
        case 'closed': {
          dataView(memory0).setInt8(arg2 + 4, 1, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant5.tag)}\` (received \`${variant5}\`) specified for \`StreamError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]input-stream.blocking-read"][Instruction::Return]', {
    funcName: '[method]input-stream.blocking-read',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline32(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable3[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable3.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.check-write"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]output-stream.check-write');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.checkWrite()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.check-write"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      dataView(memory0).setBigInt64(arg1 + 8, toUint64(e), true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      var variant4 = e;
      switch (variant4.tag) {
        case 'last-operation-failed': {
          const e = variant4.val;
          dataView(memory0).setInt8(arg1 + 8, 0, true);
          if (!(e instanceof Error$1)) {
            throw new TypeError('Resource error: Not a valid "Error" resource.');
          }
          var handle3 = e[symbolRscHandle];
          if (!handle3) {
            const rep = e[symbolRscRep] || ++captureCnt0;
            captureTable0.set(rep, e);
            handle3 = rscTableCreateOwn(handleTable0, rep);
          }
          dataView(memory0).setInt32(arg1 + 12, handle3, true);
          break;
        }
        case 'closed': {
          dataView(memory0).setInt8(arg1 + 8, 1, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant4.tag)}\` (received \`${variant4}\`) specified for \`StreamError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.check-write"][Instruction::Return]', {
    funcName: '[method]output-stream.check-write',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline33(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable3[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable3.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = new Uint8Array(memory0.buffer.slice(ptr3, ptr3 + len3 * 1));
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.write"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]output-stream.write');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.write(result3)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.write"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant6 = ret;
  switch (variant6.tag) {
    case 'ok': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg3 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      var variant5 = e;
      switch (variant5.tag) {
        case 'last-operation-failed': {
          const e = variant5.val;
          dataView(memory0).setInt8(arg3 + 4, 0, true);
          if (!(e instanceof Error$1)) {
            throw new TypeError('Resource error: Not a valid "Error" resource.');
          }
          var handle4 = e[symbolRscHandle];
          if (!handle4) {
            const rep = e[symbolRscRep] || ++captureCnt0;
            captureTable0.set(rep, e);
            handle4 = rscTableCreateOwn(handleTable0, rep);
          }
          dataView(memory0).setInt32(arg3 + 8, handle4, true);
          break;
        }
        case 'closed': {
          dataView(memory0).setInt8(arg3 + 4, 1, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant5.tag)}\` (received \`${variant5}\`) specified for \`StreamError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.write"][Instruction::Return]', {
    funcName: '[method]output-stream.write',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline34(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable3[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable3.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.blocking-flush"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]output-stream.blocking-flush');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.blockingFlush()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.blocking-flush"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      var variant4 = e;
      switch (variant4.tag) {
        case 'last-operation-failed': {
          const e = variant4.val;
          dataView(memory0).setInt8(arg1 + 4, 0, true);
          if (!(e instanceof Error$1)) {
            throw new TypeError('Resource error: Not a valid "Error" resource.');
          }
          var handle3 = e[symbolRscHandle];
          if (!handle3) {
            const rep = e[symbolRscRep] || ++captureCnt0;
            captureTable0.set(rep, e);
            handle3 = rscTableCreateOwn(handleTable0, rep);
          }
          dataView(memory0).setInt32(arg1 + 8, handle3, true);
          break;
        }
        case 'closed': {
          dataView(memory0).setInt8(arg1 + 4, 1, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant4.tag)}\` (received \`${variant4}\`) specified for \`StreamError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.blocking-flush"][Instruction::Return]', {
    funcName: '[method]output-stream.blocking-flush',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline35(arg0, arg1) {
  _debugLog('[iface="wasi:random/random@0.2.3", function="get-random-bytes"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-random-bytes');
  const ret = getRandomBytes(BigInt.asUintN(64, arg0));
  _debugLog('[iface="wasi:random/random@0.2.3", function="get-random-bytes"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var val0 = ret;
  var len0 = val0.byteLength;
  var ptr0 = realloc0(0, 0, 1, len0 * 1);
  var src0 = new Uint8Array(val0.buffer || val0, val0.byteOffset, len0 * 1);
  (new Uint8Array(memory0.buffer, ptr0, len0 * 1)).set(src0);
  dataView(memory0).setUint32(arg1 + 4, len0, true);
  dataView(memory0).setUint32(arg1 + 0, ptr0, true);
  _debugLog('[iface="wasi:random/random@0.2.3", function="get-random-bytes"][Instruction::Return]', {
    funcName: 'get-random-bytes',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline36(arg0, arg1, arg2) {
  var len2 = arg1;
  var base2 = arg0;
  var result2 = [];
  for (let i = 0; i < len2; i++) {
    const base = base2 + i * 16;
    var ptr0 = dataView(memory0).getUint32(base + 0, true);
    var len0 = dataView(memory0).getUint32(base + 4, true);
    var result0 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr0, len0));
    var ptr1 = dataView(memory0).getUint32(base + 8, true);
    var len1 = dataView(memory0).getUint32(base + 12, true);
    var result1 = new Uint8Array(memory0.buffer.slice(ptr1, ptr1 + len1 * 1));
    result2.push([result0, result1]);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]fields.from-list"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[static]fields.from-list');
  let ret;
  try {
    ret = { tag: 'ok', val: Fields.fromList(result2)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]fields.from-list"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg2 + 0, 0, true);
      if (!(e instanceof Fields)) {
        throw new TypeError('Resource error: Not a valid "Fields" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt7;
        captureTable7.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable7, rep);
      }
      dataView(memory0).setInt32(arg2 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg2 + 0, 1, true);
      var variant4 = e;
      switch (variant4.tag) {
        case 'invalid-syntax': {
          dataView(memory0).setInt8(arg2 + 4, 0, true);
          break;
        }
        case 'forbidden': {
          dataView(memory0).setInt8(arg2 + 4, 1, true);
          break;
        }
        case 'immutable': {
          dataView(memory0).setInt8(arg2 + 4, 2, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant4.tag)}\` (received \`${variant4}\`) specified for \`HeaderError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]fields.from-list"][Instruction::Return]', {
    funcName: '[static]fields.from-list',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline37(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.get"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.get');
  const ret = rsc0.get(result3);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.get"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var vec5 = ret;
  var len5 = vec5.length;
  var result5 = realloc0(0, 0, 4, len5 * 8);
  for (let i = 0; i < vec5.length; i++) {
    const e = vec5[i];
    const base = result5 + i * 8;var val4 = e;
    var len4 = val4.byteLength;
    var ptr4 = realloc0(0, 0, 1, len4 * 1);
    var src4 = new Uint8Array(val4.buffer || val4, val4.byteOffset, len4 * 1);
    (new Uint8Array(memory0.buffer, ptr4, len4 * 1)).set(src4);
    dataView(memory0).setUint32(base + 4, len4, true);
    dataView(memory0).setUint32(base + 0, ptr4, true);
  }
  dataView(memory0).setUint32(arg3 + 4, len5, true);
  dataView(memory0).setUint32(arg3 + 0, result5, true);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.get"][Instruction::Return]', {
    funcName: '[method]fields.get',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline38(arg0, arg1, arg2) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.has"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.has');
  const ret = rsc0.has(result3);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.has"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.has"][Instruction::Return]', {
    funcName: '[method]fields.has',
    paramCount: 1,
    postReturn: false
  });
  return ret ? 1 : 0;
}


function trampoline39(arg0, arg1, arg2, arg3, arg4, arg5) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
  var len5 = arg4;
  var base5 = arg3;
  var result5 = [];
  for (let i = 0; i < len5; i++) {
    const base = base5 + i * 8;
    var ptr4 = dataView(memory0).getUint32(base + 0, true);
    var len4 = dataView(memory0).getUint32(base + 4, true);
    var result4 = new Uint8Array(memory0.buffer.slice(ptr4, ptr4 + len4 * 1));
    result5.push(result4);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.set"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.set');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.set(result3, result5)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.set"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant7 = ret;
  switch (variant7.tag) {
    case 'ok': {
      const e = variant7.val;
      dataView(memory0).setInt8(arg5 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant7.val;
      dataView(memory0).setInt8(arg5 + 0, 1, true);
      var variant6 = e;
      switch (variant6.tag) {
        case 'invalid-syntax': {
          dataView(memory0).setInt8(arg5 + 1, 0, true);
          break;
        }
        case 'forbidden': {
          dataView(memory0).setInt8(arg5 + 1, 1, true);
          break;
        }
        case 'immutable': {
          dataView(memory0).setInt8(arg5 + 1, 2, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant6.tag)}\` (received \`${variant6}\`) specified for \`HeaderError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.set"][Instruction::Return]', {
    funcName: '[method]fields.set',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline40(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.delete"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.delete');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.delete(result3)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.delete"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg3 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      var variant4 = e;
      switch (variant4.tag) {
        case 'invalid-syntax': {
          dataView(memory0).setInt8(arg3 + 1, 0, true);
          break;
        }
        case 'forbidden': {
          dataView(memory0).setInt8(arg3 + 1, 1, true);
          break;
        }
        case 'immutable': {
          dataView(memory0).setInt8(arg3 + 1, 2, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant4.tag)}\` (received \`${variant4}\`) specified for \`HeaderError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.delete"][Instruction::Return]', {
    funcName: '[method]fields.delete',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline41(arg0, arg1, arg2, arg3, arg4, arg5) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
  var ptr4 = arg3;
  var len4 = arg4;
  var result4 = new Uint8Array(memory0.buffer.slice(ptr4, ptr4 + len4 * 1));
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.append"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.append');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.append(result3, result4)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.append"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant6 = ret;
  switch (variant6.tag) {
    case 'ok': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg5 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg5 + 0, 1, true);
      var variant5 = e;
      switch (variant5.tag) {
        case 'invalid-syntax': {
          dataView(memory0).setInt8(arg5 + 1, 0, true);
          break;
        }
        case 'forbidden': {
          dataView(memory0).setInt8(arg5 + 1, 1, true);
          break;
        }
        case 'immutable': {
          dataView(memory0).setInt8(arg5 + 1, 2, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant5.tag)}\` (received \`${variant5}\`) specified for \`HeaderError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.append"][Instruction::Return]', {
    funcName: '[method]fields.append',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline42(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable7[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable7.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Fields.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.entries"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]fields.entries');
  const ret = rsc0.entries();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.entries"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var vec6 = ret;
  var len6 = vec6.length;
  var result6 = realloc0(0, 0, 4, len6 * 16);
  for (let i = 0; i < vec6.length; i++) {
    const e = vec6[i];
    const base = result6 + i * 16;var [tuple3_0, tuple3_1] = e;
    var ptr4 = utf8Encode(tuple3_0, realloc0, memory0);
    var len4 = utf8EncodedLen;
    dataView(memory0).setUint32(base + 4, len4, true);
    dataView(memory0).setUint32(base + 0, ptr4, true);
    var val5 = tuple3_1;
    var len5 = val5.byteLength;
    var ptr5 = realloc0(0, 0, 1, len5 * 1);
    var src5 = new Uint8Array(val5.buffer || val5, val5.byteOffset, len5 * 1);
    (new Uint8Array(memory0.buffer, ptr5, len5 * 1)).set(src5);
    dataView(memory0).setUint32(base + 12, len5, true);
    dataView(memory0).setUint32(base + 8, ptr5, true);
  }
  dataView(memory0).setUint32(arg1 + 4, len6, true);
  dataView(memory0).setUint32(arg1 + 0, result6, true);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]fields.entries"][Instruction::Return]', {
    funcName: '[method]fields.entries',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline43(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable8[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable8.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.method"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-request.method');
  const ret = rsc0.method();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.method"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'get': {
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      break;
    }
    case 'head': {
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    case 'post': {
      dataView(memory0).setInt8(arg1 + 0, 2, true);
      break;
    }
    case 'put': {
      dataView(memory0).setInt8(arg1 + 0, 3, true);
      break;
    }
    case 'delete': {
      dataView(memory0).setInt8(arg1 + 0, 4, true);
      break;
    }
    case 'connect': {
      dataView(memory0).setInt8(arg1 + 0, 5, true);
      break;
    }
    case 'options': {
      dataView(memory0).setInt8(arg1 + 0, 6, true);
      break;
    }
    case 'trace': {
      dataView(memory0).setInt8(arg1 + 0, 7, true);
      break;
    }
    case 'patch': {
      dataView(memory0).setInt8(arg1 + 0, 8, true);
      break;
    }
    case 'other': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 9, true);
      var ptr3 = utf8Encode(e, realloc0, memory0);
      var len3 = utf8EncodedLen;
      dataView(memory0).setUint32(arg1 + 8, len3, true);
      dataView(memory0).setUint32(arg1 + 4, ptr3, true);
      break;
    }
    default: {
      throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant4.tag)}\` (received \`${variant4}\`) specified for \`Method\``);
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.method"][Instruction::Return]', {
    funcName: '[method]incoming-request.method',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline44(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable8[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable8.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.path-with-query"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-request.path-with-query');
  const ret = rsc0.pathWithQuery();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.path-with-query"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  if (variant4 === null || variant4=== undefined) {
    dataView(memory0).setInt8(arg1 + 0, 0, true);
  } else {
    const e = variant4;
    dataView(memory0).setInt8(arg1 + 0, 1, true);
    var ptr3 = utf8Encode(e, realloc0, memory0);
    var len3 = utf8EncodedLen;
    dataView(memory0).setUint32(arg1 + 8, len3, true);
    dataView(memory0).setUint32(arg1 + 4, ptr3, true);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.path-with-query"][Instruction::Return]', {
    funcName: '[method]incoming-request.path-with-query',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline45(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable8[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable8.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.scheme"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-request.scheme');
  const ret = rsc0.scheme();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.scheme"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  if (variant5 === null || variant5=== undefined) {
    dataView(memory0).setInt8(arg1 + 0, 0, true);
  } else {
    const e = variant5;
    dataView(memory0).setInt8(arg1 + 0, 1, true);
    var variant4 = e;
    switch (variant4.tag) {
      case 'HTTP': {
        dataView(memory0).setInt8(arg1 + 4, 0, true);
        break;
      }
      case 'HTTPS': {
        dataView(memory0).setInt8(arg1 + 4, 1, true);
        break;
      }
      case 'other': {
        const e = variant4.val;
        dataView(memory0).setInt8(arg1 + 4, 2, true);
        var ptr3 = utf8Encode(e, realloc0, memory0);
        var len3 = utf8EncodedLen;
        dataView(memory0).setUint32(arg1 + 12, len3, true);
        dataView(memory0).setUint32(arg1 + 8, ptr3, true);
        break;
      }
      default: {
        throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant4.tag)}\` (received \`${variant4}\`) specified for \`Scheme\``);
      }
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.scheme"][Instruction::Return]', {
    funcName: '[method]incoming-request.scheme',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline46(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable8[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable8.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.authority"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-request.authority');
  const ret = rsc0.authority();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.authority"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  if (variant4 === null || variant4=== undefined) {
    dataView(memory0).setInt8(arg1 + 0, 0, true);
  } else {
    const e = variant4;
    dataView(memory0).setInt8(arg1 + 0, 1, true);
    var ptr3 = utf8Encode(e, realloc0, memory0);
    var len3 = utf8EncodedLen;
    dataView(memory0).setUint32(arg1 + 8, len3, true);
    dataView(memory0).setUint32(arg1 + 4, ptr3, true);
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.authority"][Instruction::Return]', {
    funcName: '[method]incoming-request.authority',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable9 = [T_FLAG, 0];
const captureTable9= new Map();
let captureCnt9 = 0;
handleTables[9] = handleTable9;

function trampoline47(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable8[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable8.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.consume"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-request.consume');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.consume()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.consume"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'ok': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof IncomingBody)) {
        throw new TypeError('Resource error: Not a valid "IncomingBody" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt9;
        captureTable9.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable9, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-request.consume"][Instruction::Return]', {
    funcName: '[method]incoming-request.consume',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable11 = [T_FLAG, 0];
const captureTable11= new Map();
let captureCnt11 = 0;
handleTables[11] = handleTable11;

function trampoline48(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.body"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-request.body');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.body()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.body"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'ok': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof OutgoingBody)) {
        throw new TypeError('Resource error: Not a valid "OutgoingBody" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt11;
        captureTable11.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable11, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.body"][Instruction::Return]', {
    funcName: '[method]outgoing-request.body',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline49(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  let variant4;
  switch (arg1) {
    case 0: {
      variant4= {
        tag: 'get',
      };
      break;
    }
    case 1: {
      variant4= {
        tag: 'head',
      };
      break;
    }
    case 2: {
      variant4= {
        tag: 'post',
      };
      break;
    }
    case 3: {
      variant4= {
        tag: 'put',
      };
      break;
    }
    case 4: {
      variant4= {
        tag: 'delete',
      };
      break;
    }
    case 5: {
      variant4= {
        tag: 'connect',
      };
      break;
    }
    case 6: {
      variant4= {
        tag: 'options',
      };
      break;
    }
    case 7: {
      variant4= {
        tag: 'trace',
      };
      break;
    }
    case 8: {
      variant4= {
        tag: 'patch',
      };
      break;
    }
    case 9: {
      var ptr3 = arg2;
      var len3 = arg3;
      var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
      variant4= {
        tag: 'other',
        val: result3
      };
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for Method');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-method"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-request.set-method');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.setMethod(variant4)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-method"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  let variant5_0;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      variant5_0 = 0;
      break;
    }
    case 'err': {
      const e = variant5.val;
      variant5_0 = 1;
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-method"][Instruction::Return]', {
    funcName: '[method]outgoing-request.set-method',
    paramCount: 1,
    postReturn: false
  });
  return variant5_0;
}


function trampoline50(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  let variant4;
  switch (arg1) {
    case 0: {
      variant4 = undefined;
      break;
    }
    case 1: {
      var ptr3 = arg2;
      var len3 = arg3;
      var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
      variant4 = result3;
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for option');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-path-with-query"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-request.set-path-with-query');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.setPathWithQuery(variant4)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-path-with-query"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  let variant5_0;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      variant5_0 = 0;
      break;
    }
    case 'err': {
      const e = variant5.val;
      variant5_0 = 1;
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-path-with-query"][Instruction::Return]', {
    funcName: '[method]outgoing-request.set-path-with-query',
    paramCount: 1,
    postReturn: false
  });
  return variant5_0;
}


function trampoline51(arg0, arg1, arg2, arg3, arg4) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  let variant5;
  switch (arg1) {
    case 0: {
      variant5 = undefined;
      break;
    }
    case 1: {
      let variant4;
      switch (arg2) {
        case 0: {
          variant4= {
            tag: 'HTTP',
          };
          break;
        }
        case 1: {
          variant4= {
            tag: 'HTTPS',
          };
          break;
        }
        case 2: {
          var ptr3 = arg3;
          var len3 = arg4;
          var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
          variant4= {
            tag: 'other',
            val: result3
          };
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for Scheme');
        }
      }
      variant5 = variant4;
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for option');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-scheme"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-request.set-scheme');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.setScheme(variant5)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-scheme"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant6 = ret;
  let variant6_0;
  switch (variant6.tag) {
    case 'ok': {
      const e = variant6.val;
      variant6_0 = 0;
      break;
    }
    case 'err': {
      const e = variant6.val;
      variant6_0 = 1;
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-scheme"][Instruction::Return]', {
    funcName: '[method]outgoing-request.set-scheme',
    paramCount: 1,
    postReturn: false
  });
  return variant6_0;
}


function trampoline52(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  let variant4;
  switch (arg1) {
    case 0: {
      variant4 = undefined;
      break;
    }
    case 1: {
      var ptr3 = arg2;
      var len3 = arg3;
      var result3 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr3, len3));
      variant4 = result3;
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for option');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-authority"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-request.set-authority');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.setAuthority(variant4)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-authority"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  let variant5_0;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      variant5_0 = 0;
      break;
    }
    case 'err': {
      const e = variant5.val;
      variant5_0 = 1;
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-request.set-authority"][Instruction::Return]', {
    funcName: '[method]outgoing-request.set-authority',
    paramCount: 1,
    postReturn: false
  });
  return variant5_0;
}

const handleTable12 = [T_FLAG, 0];
const captureTable12= new Map();
let captureCnt12 = 0;
handleTables[12] = handleTable12;

function trampoline53(arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
  var handle1 = arg0;
  var rep2 = handleTable12[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable12.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(ResponseOutparam.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  else {
    captureTable12.delete(rep2);
  }
  rscTableRemove(handleTable12, handle1);
  let variant38;
  switch (arg1) {
    case 0: {
      var handle4 = arg2;
      var rep5 = handleTable13[(handle4 << 1) + 1] & ~T_FLAG;
      var rsc3 = captureTable13.get(rep5);
      if (!rsc3) {
        rsc3 = Object.create(OutgoingResponse.prototype);
        Object.defineProperty(rsc3, symbolRscHandle, { writable: true, value: handle4});
        Object.defineProperty(rsc3, symbolRscRep, { writable: true, value: rep5});
      }
      else {
        captureTable13.delete(rep5);
      }
      rscTableRemove(handleTable13, handle4);
      variant38= {
        tag: 'ok',
        val: rsc3
      };
      break;
    }
    case 1: {
      let variant37;
      switch (arg2) {
        case 0: {
          variant37= {
            tag: 'DNS-timeout',
          };
          break;
        }
        case 1: {
          let variant7;
          switch (arg3) {
            case 0: {
              variant7 = undefined;
              break;
            }
            case 1: {
              var ptr6 = Number(arg4);
              var len6 = arg5;
              var result6 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr6, len6));
              variant7 = result6;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          let variant8;
          switch (arg6) {
            case 0: {
              variant8 = undefined;
              break;
            }
            case 1: {
              variant8 = clampGuest(arg7, 0, 65535);
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'DNS-error',
            val: {
              rcode: variant7,
              infoCode: variant8,
            }
          };
          break;
        }
        case 2: {
          variant37= {
            tag: 'destination-not-found',
          };
          break;
        }
        case 3: {
          variant37= {
            tag: 'destination-unavailable',
          };
          break;
        }
        case 4: {
          variant37= {
            tag: 'destination-IP-prohibited',
          };
          break;
        }
        case 5: {
          variant37= {
            tag: 'destination-IP-unroutable',
          };
          break;
        }
        case 6: {
          variant37= {
            tag: 'connection-refused',
          };
          break;
        }
        case 7: {
          variant37= {
            tag: 'connection-terminated',
          };
          break;
        }
        case 8: {
          variant37= {
            tag: 'connection-timeout',
          };
          break;
        }
        case 9: {
          variant37= {
            tag: 'connection-read-timeout',
          };
          break;
        }
        case 10: {
          variant37= {
            tag: 'connection-write-timeout',
          };
          break;
        }
        case 11: {
          variant37= {
            tag: 'connection-limit-reached',
          };
          break;
        }
        case 12: {
          variant37= {
            tag: 'TLS-protocol-error',
          };
          break;
        }
        case 13: {
          variant37= {
            tag: 'TLS-certificate-error',
          };
          break;
        }
        case 14: {
          let variant9;
          switch (arg3) {
            case 0: {
              variant9 = undefined;
              break;
            }
            case 1: {
              variant9 = clampGuest(Number(arg4), 0, 255);
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          let variant11;
          switch (arg5) {
            case 0: {
              variant11 = undefined;
              break;
            }
            case 1: {
              var ptr10 = arg6;
              var len10 = arg7;
              var result10 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr10, len10));
              variant11 = result10;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'TLS-alert-received',
            val: {
              alertId: variant9,
              alertMessage: variant11,
            }
          };
          break;
        }
        case 15: {
          variant37= {
            tag: 'HTTP-request-denied',
          };
          break;
        }
        case 16: {
          variant37= {
            tag: 'HTTP-request-length-required',
          };
          break;
        }
        case 17: {
          let variant12;
          switch (arg3) {
            case 0: {
              variant12 = undefined;
              break;
            }
            case 1: {
              variant12 = BigInt.asUintN(64, arg4);
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-request-body-size',
            val: variant12
          };
          break;
        }
        case 18: {
          variant37= {
            tag: 'HTTP-request-method-invalid',
          };
          break;
        }
        case 19: {
          variant37= {
            tag: 'HTTP-request-URI-invalid',
          };
          break;
        }
        case 20: {
          variant37= {
            tag: 'HTTP-request-URI-too-long',
          };
          break;
        }
        case 21: {
          let variant13;
          switch (arg3) {
            case 0: {
              variant13 = undefined;
              break;
            }
            case 1: {
              variant13 = Number(arg4) >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-request-header-section-size',
            val: variant13
          };
          break;
        }
        case 22: {
          let variant17;
          switch (arg3) {
            case 0: {
              variant17 = undefined;
              break;
            }
            case 1: {
              let variant15;
              switch (Number(arg4)) {
                case 0: {
                  variant15 = undefined;
                  break;
                }
                case 1: {
                  var ptr14 = arg5;
                  var len14 = arg6;
                  var result14 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr14, len14));
                  variant15 = result14;
                  break;
                }
                default: {
                  throw new TypeError('invalid variant discriminant for option');
                }
              }
              let variant16;
              switch (arg7) {
                case 0: {
                  variant16 = undefined;
                  break;
                }
                case 1: {
                  variant16 = arg8 >>> 0;
                  break;
                }
                default: {
                  throw new TypeError('invalid variant discriminant for option');
                }
              }
              variant17 = {
                fieldName: variant15,
                fieldSize: variant16,
              };
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-request-header-size',
            val: variant17
          };
          break;
        }
        case 23: {
          let variant18;
          switch (arg3) {
            case 0: {
              variant18 = undefined;
              break;
            }
            case 1: {
              variant18 = Number(arg4) >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-request-trailer-section-size',
            val: variant18
          };
          break;
        }
        case 24: {
          let variant20;
          switch (arg3) {
            case 0: {
              variant20 = undefined;
              break;
            }
            case 1: {
              var ptr19 = Number(arg4);
              var len19 = arg5;
              var result19 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr19, len19));
              variant20 = result19;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          let variant21;
          switch (arg6) {
            case 0: {
              variant21 = undefined;
              break;
            }
            case 1: {
              variant21 = arg7 >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-request-trailer-size',
            val: {
              fieldName: variant20,
              fieldSize: variant21,
            }
          };
          break;
        }
        case 25: {
          variant37= {
            tag: 'HTTP-response-incomplete',
          };
          break;
        }
        case 26: {
          let variant22;
          switch (arg3) {
            case 0: {
              variant22 = undefined;
              break;
            }
            case 1: {
              variant22 = Number(arg4) >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-header-section-size',
            val: variant22
          };
          break;
        }
        case 27: {
          let variant24;
          switch (arg3) {
            case 0: {
              variant24 = undefined;
              break;
            }
            case 1: {
              var ptr23 = Number(arg4);
              var len23 = arg5;
              var result23 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr23, len23));
              variant24 = result23;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          let variant25;
          switch (arg6) {
            case 0: {
              variant25 = undefined;
              break;
            }
            case 1: {
              variant25 = arg7 >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-header-size',
            val: {
              fieldName: variant24,
              fieldSize: variant25,
            }
          };
          break;
        }
        case 28: {
          let variant26;
          switch (arg3) {
            case 0: {
              variant26 = undefined;
              break;
            }
            case 1: {
              variant26 = BigInt.asUintN(64, arg4);
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-body-size',
            val: variant26
          };
          break;
        }
        case 29: {
          let variant27;
          switch (arg3) {
            case 0: {
              variant27 = undefined;
              break;
            }
            case 1: {
              variant27 = Number(arg4) >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-trailer-section-size',
            val: variant27
          };
          break;
        }
        case 30: {
          let variant29;
          switch (arg3) {
            case 0: {
              variant29 = undefined;
              break;
            }
            case 1: {
              var ptr28 = Number(arg4);
              var len28 = arg5;
              var result28 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr28, len28));
              variant29 = result28;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          let variant30;
          switch (arg6) {
            case 0: {
              variant30 = undefined;
              break;
            }
            case 1: {
              variant30 = arg7 >>> 0;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-trailer-size',
            val: {
              fieldName: variant29,
              fieldSize: variant30,
            }
          };
          break;
        }
        case 31: {
          let variant32;
          switch (arg3) {
            case 0: {
              variant32 = undefined;
              break;
            }
            case 1: {
              var ptr31 = Number(arg4);
              var len31 = arg5;
              var result31 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr31, len31));
              variant32 = result31;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-transfer-coding',
            val: variant32
          };
          break;
        }
        case 32: {
          let variant34;
          switch (arg3) {
            case 0: {
              variant34 = undefined;
              break;
            }
            case 1: {
              var ptr33 = Number(arg4);
              var len33 = arg5;
              var result33 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr33, len33));
              variant34 = result33;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'HTTP-response-content-coding',
            val: variant34
          };
          break;
        }
        case 33: {
          variant37= {
            tag: 'HTTP-response-timeout',
          };
          break;
        }
        case 34: {
          variant37= {
            tag: 'HTTP-upgrade-failed',
          };
          break;
        }
        case 35: {
          variant37= {
            tag: 'HTTP-protocol-error',
          };
          break;
        }
        case 36: {
          variant37= {
            tag: 'loop-detected',
          };
          break;
        }
        case 37: {
          variant37= {
            tag: 'configuration-error',
          };
          break;
        }
        case 38: {
          let variant36;
          switch (arg3) {
            case 0: {
              variant36 = undefined;
              break;
            }
            case 1: {
              var ptr35 = Number(arg4);
              var len35 = arg5;
              var result35 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr35, len35));
              variant36 = result35;
              break;
            }
            default: {
              throw new TypeError('invalid variant discriminant for option');
            }
          }
          variant37= {
            tag: 'internal-error',
            val: variant36
          };
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for ErrorCode');
        }
      }
      variant38= {
        tag: 'err',
        val: variant37
      };
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for expected');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]response-outparam.set"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[static]response-outparam.set');
  ResponseOutparam.set(rsc0, variant38);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]response-outparam.set"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]response-outparam.set"][Instruction::Return]', {
    funcName: '[static]response-outparam.set',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline54(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable14[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable14.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.consume"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-response.consume');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.consume()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.consume"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'ok': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof IncomingBody)) {
        throw new TypeError('Resource error: Not a valid "IncomingBody" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt9;
        captureTable9.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable9, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-response.consume"][Instruction::Return]', {
    funcName: '[method]incoming-response.consume',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline55(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable9[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable9.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(IncomingBody.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-body.stream"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]incoming-body.stream');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.stream()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-body.stream"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'ok': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof InputStream)) {
        throw new TypeError('Resource error: Not a valid "InputStream" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt2;
        captureTable2.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable2, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]incoming-body.stream"][Instruction::Return]', {
    funcName: '[method]incoming-body.stream',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline56(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable13[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable13.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.body"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-response.body');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.body()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.body"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'ok': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof OutgoingBody)) {
        throw new TypeError('Resource error: Not a valid "OutgoingBody" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt11;
        captureTable11.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable11, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-response.body"][Instruction::Return]', {
    funcName: '[method]outgoing-response.body',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline57(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable11[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable11.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingBody.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-body.write"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]outgoing-body.write');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.write()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-body.write"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  switch (variant4.tag) {
    case 'ok': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof OutputStream)) {
        throw new TypeError('Resource error: Not a valid "OutputStream" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt3;
        captureTable3.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable3, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant4.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]outgoing-body.write"][Instruction::Return]', {
    funcName: '[method]outgoing-body.write',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline58(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable11[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable11.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingBody.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  else {
    captureTable11.delete(rep2);
  }
  rscTableRemove(handleTable11, handle1);
  let variant6;
  switch (arg1) {
    case 0: {
      variant6 = undefined;
      break;
    }
    case 1: {
      var handle4 = arg2;
      var rep5 = handleTable7[(handle4 << 1) + 1] & ~T_FLAG;
      var rsc3 = captureTable7.get(rep5);
      if (!rsc3) {
        rsc3 = Object.create(Fields.prototype);
        Object.defineProperty(rsc3, symbolRscHandle, { writable: true, value: handle4});
        Object.defineProperty(rsc3, symbolRscRep, { writable: true, value: rep5});
      }
      else {
        captureTable7.delete(rep5);
      }
      rscTableRemove(handleTable7, handle4);
      variant6 = rsc3;
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for option');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]outgoing-body.finish"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[static]outgoing-body.finish');
  let ret;
  try {
    ret = { tag: 'ok', val: OutgoingBody.finish(rsc0, variant6)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]outgoing-body.finish"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var variant45 = ret;
  switch (variant45.tag) {
    case 'ok': {
      const e = variant45.val;
      dataView(memory0).setInt8(arg3 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant45.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      var variant44 = e;
      switch (variant44.tag) {
        case 'DNS-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 0, true);
          break;
        }
        case 'DNS-error': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 1, true);
          var {rcode: v7_0, infoCode: v7_1 } = e;
          var variant9 = v7_0;
          if (variant9 === null || variant9=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant9;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr8 = utf8Encode(e, realloc0, memory0);
            var len8 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len8, true);
            dataView(memory0).setUint32(arg3 + 20, ptr8, true);
          }
          var variant10 = v7_1;
          if (variant10 === null || variant10=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant10;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt16(arg3 + 30, toUint16(e), true);
          }
          break;
        }
        case 'destination-not-found': {
          dataView(memory0).setInt8(arg3 + 8, 2, true);
          break;
        }
        case 'destination-unavailable': {
          dataView(memory0).setInt8(arg3 + 8, 3, true);
          break;
        }
        case 'destination-IP-prohibited': {
          dataView(memory0).setInt8(arg3 + 8, 4, true);
          break;
        }
        case 'destination-IP-unroutable': {
          dataView(memory0).setInt8(arg3 + 8, 5, true);
          break;
        }
        case 'connection-refused': {
          dataView(memory0).setInt8(arg3 + 8, 6, true);
          break;
        }
        case 'connection-terminated': {
          dataView(memory0).setInt8(arg3 + 8, 7, true);
          break;
        }
        case 'connection-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 8, true);
          break;
        }
        case 'connection-read-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 9, true);
          break;
        }
        case 'connection-write-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 10, true);
          break;
        }
        case 'connection-limit-reached': {
          dataView(memory0).setInt8(arg3 + 8, 11, true);
          break;
        }
        case 'TLS-protocol-error': {
          dataView(memory0).setInt8(arg3 + 8, 12, true);
          break;
        }
        case 'TLS-certificate-error': {
          dataView(memory0).setInt8(arg3 + 8, 13, true);
          break;
        }
        case 'TLS-alert-received': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 14, true);
          var {alertId: v11_0, alertMessage: v11_1 } = e;
          var variant12 = v11_0;
          if (variant12 === null || variant12=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant12;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt8(arg3 + 17, toUint8(e), true);
          }
          var variant14 = v11_1;
          if (variant14 === null || variant14=== undefined) {
            dataView(memory0).setInt8(arg3 + 20, 0, true);
          } else {
            const e = variant14;
            dataView(memory0).setInt8(arg3 + 20, 1, true);
            var ptr13 = utf8Encode(e, realloc0, memory0);
            var len13 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 28, len13, true);
            dataView(memory0).setUint32(arg3 + 24, ptr13, true);
          }
          break;
        }
        case 'HTTP-request-denied': {
          dataView(memory0).setInt8(arg3 + 8, 15, true);
          break;
        }
        case 'HTTP-request-length-required': {
          dataView(memory0).setInt8(arg3 + 8, 16, true);
          break;
        }
        case 'HTTP-request-body-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 17, true);
          var variant15 = e;
          if (variant15 === null || variant15=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant15;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setBigInt64(arg3 + 24, toUint64(e), true);
          }
          break;
        }
        case 'HTTP-request-method-invalid': {
          dataView(memory0).setInt8(arg3 + 8, 18, true);
          break;
        }
        case 'HTTP-request-URI-invalid': {
          dataView(memory0).setInt8(arg3 + 8, 19, true);
          break;
        }
        case 'HTTP-request-URI-too-long': {
          dataView(memory0).setInt8(arg3 + 8, 20, true);
          break;
        }
        case 'HTTP-request-header-section-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 21, true);
          var variant16 = e;
          if (variant16 === null || variant16=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant16;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-request-header-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 22, true);
          var variant21 = e;
          if (variant21 === null || variant21=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant21;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var {fieldName: v17_0, fieldSize: v17_1 } = e;
            var variant19 = v17_0;
            if (variant19 === null || variant19=== undefined) {
              dataView(memory0).setInt8(arg3 + 20, 0, true);
            } else {
              const e = variant19;
              dataView(memory0).setInt8(arg3 + 20, 1, true);
              var ptr18 = utf8Encode(e, realloc0, memory0);
              var len18 = utf8EncodedLen;
              dataView(memory0).setUint32(arg3 + 28, len18, true);
              dataView(memory0).setUint32(arg3 + 24, ptr18, true);
            }
            var variant20 = v17_1;
            if (variant20 === null || variant20=== undefined) {
              dataView(memory0).setInt8(arg3 + 32, 0, true);
            } else {
              const e = variant20;
              dataView(memory0).setInt8(arg3 + 32, 1, true);
              dataView(memory0).setInt32(arg3 + 36, toUint32(e), true);
            }
          }
          break;
        }
        case 'HTTP-request-trailer-section-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 23, true);
          var variant22 = e;
          if (variant22 === null || variant22=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant22;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-request-trailer-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 24, true);
          var {fieldName: v23_0, fieldSize: v23_1 } = e;
          var variant25 = v23_0;
          if (variant25 === null || variant25=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant25;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr24 = utf8Encode(e, realloc0, memory0);
            var len24 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len24, true);
            dataView(memory0).setUint32(arg3 + 20, ptr24, true);
          }
          var variant26 = v23_1;
          if (variant26 === null || variant26=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant26;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt32(arg3 + 32, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-incomplete': {
          dataView(memory0).setInt8(arg3 + 8, 25, true);
          break;
        }
        case 'HTTP-response-header-section-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 26, true);
          var variant27 = e;
          if (variant27 === null || variant27=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant27;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-header-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 27, true);
          var {fieldName: v28_0, fieldSize: v28_1 } = e;
          var variant30 = v28_0;
          if (variant30 === null || variant30=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant30;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr29 = utf8Encode(e, realloc0, memory0);
            var len29 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len29, true);
            dataView(memory0).setUint32(arg3 + 20, ptr29, true);
          }
          var variant31 = v28_1;
          if (variant31 === null || variant31=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant31;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt32(arg3 + 32, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-body-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 28, true);
          var variant32 = e;
          if (variant32 === null || variant32=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant32;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setBigInt64(arg3 + 24, toUint64(e), true);
          }
          break;
        }
        case 'HTTP-response-trailer-section-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 29, true);
          var variant33 = e;
          if (variant33 === null || variant33=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant33;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-trailer-size': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 30, true);
          var {fieldName: v34_0, fieldSize: v34_1 } = e;
          var variant36 = v34_0;
          if (variant36 === null || variant36=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant36;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr35 = utf8Encode(e, realloc0, memory0);
            var len35 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len35, true);
            dataView(memory0).setUint32(arg3 + 20, ptr35, true);
          }
          var variant37 = v34_1;
          if (variant37 === null || variant37=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant37;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt32(arg3 + 32, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-transfer-coding': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 31, true);
          var variant39 = e;
          if (variant39 === null || variant39=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant39;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr38 = utf8Encode(e, realloc0, memory0);
            var len38 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len38, true);
            dataView(memory0).setUint32(arg3 + 20, ptr38, true);
          }
          break;
        }
        case 'HTTP-response-content-coding': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 32, true);
          var variant41 = e;
          if (variant41 === null || variant41=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant41;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr40 = utf8Encode(e, realloc0, memory0);
            var len40 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len40, true);
            dataView(memory0).setUint32(arg3 + 20, ptr40, true);
          }
          break;
        }
        case 'HTTP-response-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 33, true);
          break;
        }
        case 'HTTP-upgrade-failed': {
          dataView(memory0).setInt8(arg3 + 8, 34, true);
          break;
        }
        case 'HTTP-protocol-error': {
          dataView(memory0).setInt8(arg3 + 8, 35, true);
          break;
        }
        case 'loop-detected': {
          dataView(memory0).setInt8(arg3 + 8, 36, true);
          break;
        }
        case 'configuration-error': {
          dataView(memory0).setInt8(arg3 + 8, 37, true);
          break;
        }
        case 'internal-error': {
          const e = variant44.val;
          dataView(memory0).setInt8(arg3 + 8, 38, true);
          var variant43 = e;
          if (variant43 === null || variant43=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant43;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr42 = utf8Encode(e, realloc0, memory0);
            var len42 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len42, true);
            dataView(memory0).setUint32(arg3 + 20, ptr42, true);
          }
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant44.tag)}\` (received \`${variant44}\`) specified for \`ErrorCode\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[static]outgoing-body.finish"][Instruction::Return]', {
    funcName: '[static]outgoing-body.finish',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline59(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable15[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable15.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(FutureIncomingResponse.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]future-incoming-response.get"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]future-incoming-response.get');
  const ret = rsc0.get();
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]future-incoming-response.get"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant44 = ret;
  if (variant44 === null || variant44=== undefined) {
    dataView(memory0).setInt8(arg1 + 0, 0, true);
  } else {
    const e = variant44;
    dataView(memory0).setInt8(arg1 + 0, 1, true);
    var variant43 = e;
    switch (variant43.tag) {
      case 'ok': {
        const e = variant43.val;
        dataView(memory0).setInt8(arg1 + 8, 0, true);
        var variant42 = e;
        switch (variant42.tag) {
          case 'ok': {
            const e = variant42.val;
            dataView(memory0).setInt8(arg1 + 16, 0, true);
            if (!(e instanceof IncomingResponse)) {
              throw new TypeError('Resource error: Not a valid "IncomingResponse" resource.');
            }
            var handle3 = e[symbolRscHandle];
            if (!handle3) {
              const rep = e[symbolRscRep] || ++captureCnt14;
              captureTable14.set(rep, e);
              handle3 = rscTableCreateOwn(handleTable14, rep);
            }
            dataView(memory0).setInt32(arg1 + 24, handle3, true);
            break;
          }
          case 'err': {
            const e = variant42.val;
            dataView(memory0).setInt8(arg1 + 16, 1, true);
            var variant41 = e;
            switch (variant41.tag) {
              case 'DNS-timeout': {
                dataView(memory0).setInt8(arg1 + 24, 0, true);
                break;
              }
              case 'DNS-error': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 1, true);
                var {rcode: v4_0, infoCode: v4_1 } = e;
                var variant6 = v4_0;
                if (variant6 === null || variant6=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant6;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr5 = utf8Encode(e, realloc0, memory0);
                  var len5 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len5, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr5, true);
                }
                var variant7 = v4_1;
                if (variant7 === null || variant7=== undefined) {
                  dataView(memory0).setInt8(arg1 + 44, 0, true);
                } else {
                  const e = variant7;
                  dataView(memory0).setInt8(arg1 + 44, 1, true);
                  dataView(memory0).setInt16(arg1 + 46, toUint16(e), true);
                }
                break;
              }
              case 'destination-not-found': {
                dataView(memory0).setInt8(arg1 + 24, 2, true);
                break;
              }
              case 'destination-unavailable': {
                dataView(memory0).setInt8(arg1 + 24, 3, true);
                break;
              }
              case 'destination-IP-prohibited': {
                dataView(memory0).setInt8(arg1 + 24, 4, true);
                break;
              }
              case 'destination-IP-unroutable': {
                dataView(memory0).setInt8(arg1 + 24, 5, true);
                break;
              }
              case 'connection-refused': {
                dataView(memory0).setInt8(arg1 + 24, 6, true);
                break;
              }
              case 'connection-terminated': {
                dataView(memory0).setInt8(arg1 + 24, 7, true);
                break;
              }
              case 'connection-timeout': {
                dataView(memory0).setInt8(arg1 + 24, 8, true);
                break;
              }
              case 'connection-read-timeout': {
                dataView(memory0).setInt8(arg1 + 24, 9, true);
                break;
              }
              case 'connection-write-timeout': {
                dataView(memory0).setInt8(arg1 + 24, 10, true);
                break;
              }
              case 'connection-limit-reached': {
                dataView(memory0).setInt8(arg1 + 24, 11, true);
                break;
              }
              case 'TLS-protocol-error': {
                dataView(memory0).setInt8(arg1 + 24, 12, true);
                break;
              }
              case 'TLS-certificate-error': {
                dataView(memory0).setInt8(arg1 + 24, 13, true);
                break;
              }
              case 'TLS-alert-received': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 14, true);
                var {alertId: v8_0, alertMessage: v8_1 } = e;
                var variant9 = v8_0;
                if (variant9 === null || variant9=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant9;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setInt8(arg1 + 33, toUint8(e), true);
                }
                var variant11 = v8_1;
                if (variant11 === null || variant11=== undefined) {
                  dataView(memory0).setInt8(arg1 + 36, 0, true);
                } else {
                  const e = variant11;
                  dataView(memory0).setInt8(arg1 + 36, 1, true);
                  var ptr10 = utf8Encode(e, realloc0, memory0);
                  var len10 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 44, len10, true);
                  dataView(memory0).setUint32(arg1 + 40, ptr10, true);
                }
                break;
              }
              case 'HTTP-request-denied': {
                dataView(memory0).setInt8(arg1 + 24, 15, true);
                break;
              }
              case 'HTTP-request-length-required': {
                dataView(memory0).setInt8(arg1 + 24, 16, true);
                break;
              }
              case 'HTTP-request-body-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 17, true);
                var variant12 = e;
                if (variant12 === null || variant12=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant12;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setBigInt64(arg1 + 40, toUint64(e), true);
                }
                break;
              }
              case 'HTTP-request-method-invalid': {
                dataView(memory0).setInt8(arg1 + 24, 18, true);
                break;
              }
              case 'HTTP-request-URI-invalid': {
                dataView(memory0).setInt8(arg1 + 24, 19, true);
                break;
              }
              case 'HTTP-request-URI-too-long': {
                dataView(memory0).setInt8(arg1 + 24, 20, true);
                break;
              }
              case 'HTTP-request-header-section-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 21, true);
                var variant13 = e;
                if (variant13 === null || variant13=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant13;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setInt32(arg1 + 36, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-request-header-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 22, true);
                var variant18 = e;
                if (variant18 === null || variant18=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant18;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var {fieldName: v14_0, fieldSize: v14_1 } = e;
                  var variant16 = v14_0;
                  if (variant16 === null || variant16=== undefined) {
                    dataView(memory0).setInt8(arg1 + 36, 0, true);
                  } else {
                    const e = variant16;
                    dataView(memory0).setInt8(arg1 + 36, 1, true);
                    var ptr15 = utf8Encode(e, realloc0, memory0);
                    var len15 = utf8EncodedLen;
                    dataView(memory0).setUint32(arg1 + 44, len15, true);
                    dataView(memory0).setUint32(arg1 + 40, ptr15, true);
                  }
                  var variant17 = v14_1;
                  if (variant17 === null || variant17=== undefined) {
                    dataView(memory0).setInt8(arg1 + 48, 0, true);
                  } else {
                    const e = variant17;
                    dataView(memory0).setInt8(arg1 + 48, 1, true);
                    dataView(memory0).setInt32(arg1 + 52, toUint32(e), true);
                  }
                }
                break;
              }
              case 'HTTP-request-trailer-section-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 23, true);
                var variant19 = e;
                if (variant19 === null || variant19=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant19;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setInt32(arg1 + 36, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-request-trailer-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 24, true);
                var {fieldName: v20_0, fieldSize: v20_1 } = e;
                var variant22 = v20_0;
                if (variant22 === null || variant22=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant22;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr21 = utf8Encode(e, realloc0, memory0);
                  var len21 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len21, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr21, true);
                }
                var variant23 = v20_1;
                if (variant23 === null || variant23=== undefined) {
                  dataView(memory0).setInt8(arg1 + 44, 0, true);
                } else {
                  const e = variant23;
                  dataView(memory0).setInt8(arg1 + 44, 1, true);
                  dataView(memory0).setInt32(arg1 + 48, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-response-incomplete': {
                dataView(memory0).setInt8(arg1 + 24, 25, true);
                break;
              }
              case 'HTTP-response-header-section-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 26, true);
                var variant24 = e;
                if (variant24 === null || variant24=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant24;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setInt32(arg1 + 36, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-response-header-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 27, true);
                var {fieldName: v25_0, fieldSize: v25_1 } = e;
                var variant27 = v25_0;
                if (variant27 === null || variant27=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant27;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr26 = utf8Encode(e, realloc0, memory0);
                  var len26 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len26, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr26, true);
                }
                var variant28 = v25_1;
                if (variant28 === null || variant28=== undefined) {
                  dataView(memory0).setInt8(arg1 + 44, 0, true);
                } else {
                  const e = variant28;
                  dataView(memory0).setInt8(arg1 + 44, 1, true);
                  dataView(memory0).setInt32(arg1 + 48, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-response-body-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 28, true);
                var variant29 = e;
                if (variant29 === null || variant29=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant29;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setBigInt64(arg1 + 40, toUint64(e), true);
                }
                break;
              }
              case 'HTTP-response-trailer-section-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 29, true);
                var variant30 = e;
                if (variant30 === null || variant30=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant30;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  dataView(memory0).setInt32(arg1 + 36, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-response-trailer-size': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 30, true);
                var {fieldName: v31_0, fieldSize: v31_1 } = e;
                var variant33 = v31_0;
                if (variant33 === null || variant33=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant33;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr32 = utf8Encode(e, realloc0, memory0);
                  var len32 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len32, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr32, true);
                }
                var variant34 = v31_1;
                if (variant34 === null || variant34=== undefined) {
                  dataView(memory0).setInt8(arg1 + 44, 0, true);
                } else {
                  const e = variant34;
                  dataView(memory0).setInt8(arg1 + 44, 1, true);
                  dataView(memory0).setInt32(arg1 + 48, toUint32(e), true);
                }
                break;
              }
              case 'HTTP-response-transfer-coding': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 31, true);
                var variant36 = e;
                if (variant36 === null || variant36=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant36;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr35 = utf8Encode(e, realloc0, memory0);
                  var len35 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len35, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr35, true);
                }
                break;
              }
              case 'HTTP-response-content-coding': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 32, true);
                var variant38 = e;
                if (variant38 === null || variant38=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant38;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr37 = utf8Encode(e, realloc0, memory0);
                  var len37 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len37, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr37, true);
                }
                break;
              }
              case 'HTTP-response-timeout': {
                dataView(memory0).setInt8(arg1 + 24, 33, true);
                break;
              }
              case 'HTTP-upgrade-failed': {
                dataView(memory0).setInt8(arg1 + 24, 34, true);
                break;
              }
              case 'HTTP-protocol-error': {
                dataView(memory0).setInt8(arg1 + 24, 35, true);
                break;
              }
              case 'loop-detected': {
                dataView(memory0).setInt8(arg1 + 24, 36, true);
                break;
              }
              case 'configuration-error': {
                dataView(memory0).setInt8(arg1 + 24, 37, true);
                break;
              }
              case 'internal-error': {
                const e = variant41.val;
                dataView(memory0).setInt8(arg1 + 24, 38, true);
                var variant40 = e;
                if (variant40 === null || variant40=== undefined) {
                  dataView(memory0).setInt8(arg1 + 32, 0, true);
                } else {
                  const e = variant40;
                  dataView(memory0).setInt8(arg1 + 32, 1, true);
                  var ptr39 = utf8Encode(e, realloc0, memory0);
                  var len39 = utf8EncodedLen;
                  dataView(memory0).setUint32(arg1 + 40, len39, true);
                  dataView(memory0).setUint32(arg1 + 36, ptr39, true);
                }
                break;
              }
              default: {
                throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant41.tag)}\` (received \`${variant41}\`) specified for \`ErrorCode\``);
              }
            }
            break;
          }
          default: {
            throw new TypeError('invalid variant specified for result');
          }
        }
        break;
      }
      case 'err': {
        const e = variant43.val;
        dataView(memory0).setInt8(arg1 + 8, 1, true);
        break;
      }
      default: {
        throw new TypeError('invalid variant specified for result');
      }
    }
  }
  _debugLog('[iface="wasi:http/types@0.2.3", function="[method]future-incoming-response.get"][Instruction::Return]', {
    funcName: '[method]future-incoming-response.get',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable16 = [T_FLAG, 0];
const captureTable16= new Map();
let captureCnt16 = 0;
handleTables[16] = handleTable16;

function trampoline60(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable10[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable10.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutgoingRequest.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  else {
    captureTable10.delete(rep2);
  }
  rscTableRemove(handleTable10, handle1);
  let variant6;
  switch (arg1) {
    case 0: {
      variant6 = undefined;
      break;
    }
    case 1: {
      var handle4 = arg2;
      var rep5 = handleTable16[(handle4 << 1) + 1] & ~T_FLAG;
      var rsc3 = captureTable16.get(rep5);
      if (!rsc3) {
        rsc3 = Object.create(RequestOptions.prototype);
        Object.defineProperty(rsc3, symbolRscHandle, { writable: true, value: handle4});
        Object.defineProperty(rsc3, symbolRscRep, { writable: true, value: rep5});
      }
      else {
        captureTable16.delete(rep5);
      }
      rscTableRemove(handleTable16, handle4);
      variant6 = rsc3;
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for option');
    }
  }
  _debugLog('[iface="wasi:http/outgoing-handler@0.2.3", function="handle"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'handle');
  let ret;
  try {
    ret = { tag: 'ok', val: handle(rsc0, variant6)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:http/outgoing-handler@0.2.3", function="handle"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var variant46 = ret;
  switch (variant46.tag) {
    case 'ok': {
      const e = variant46.val;
      dataView(memory0).setInt8(arg3 + 0, 0, true);
      if (!(e instanceof FutureIncomingResponse)) {
        throw new TypeError('Resource error: Not a valid "FutureIncomingResponse" resource.');
      }
      var handle7 = e[symbolRscHandle];
      if (!handle7) {
        const rep = e[symbolRscRep] || ++captureCnt15;
        captureTable15.set(rep, e);
        handle7 = rscTableCreateOwn(handleTable15, rep);
      }
      dataView(memory0).setInt32(arg3 + 8, handle7, true);
      break;
    }
    case 'err': {
      const e = variant46.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      var variant45 = e;
      switch (variant45.tag) {
        case 'DNS-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 0, true);
          break;
        }
        case 'DNS-error': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 1, true);
          var {rcode: v8_0, infoCode: v8_1 } = e;
          var variant10 = v8_0;
          if (variant10 === null || variant10=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant10;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr9 = utf8Encode(e, realloc0, memory0);
            var len9 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len9, true);
            dataView(memory0).setUint32(arg3 + 20, ptr9, true);
          }
          var variant11 = v8_1;
          if (variant11 === null || variant11=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant11;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt16(arg3 + 30, toUint16(e), true);
          }
          break;
        }
        case 'destination-not-found': {
          dataView(memory0).setInt8(arg3 + 8, 2, true);
          break;
        }
        case 'destination-unavailable': {
          dataView(memory0).setInt8(arg3 + 8, 3, true);
          break;
        }
        case 'destination-IP-prohibited': {
          dataView(memory0).setInt8(arg3 + 8, 4, true);
          break;
        }
        case 'destination-IP-unroutable': {
          dataView(memory0).setInt8(arg3 + 8, 5, true);
          break;
        }
        case 'connection-refused': {
          dataView(memory0).setInt8(arg3 + 8, 6, true);
          break;
        }
        case 'connection-terminated': {
          dataView(memory0).setInt8(arg3 + 8, 7, true);
          break;
        }
        case 'connection-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 8, true);
          break;
        }
        case 'connection-read-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 9, true);
          break;
        }
        case 'connection-write-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 10, true);
          break;
        }
        case 'connection-limit-reached': {
          dataView(memory0).setInt8(arg3 + 8, 11, true);
          break;
        }
        case 'TLS-protocol-error': {
          dataView(memory0).setInt8(arg3 + 8, 12, true);
          break;
        }
        case 'TLS-certificate-error': {
          dataView(memory0).setInt8(arg3 + 8, 13, true);
          break;
        }
        case 'TLS-alert-received': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 14, true);
          var {alertId: v12_0, alertMessage: v12_1 } = e;
          var variant13 = v12_0;
          if (variant13 === null || variant13=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant13;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt8(arg3 + 17, toUint8(e), true);
          }
          var variant15 = v12_1;
          if (variant15 === null || variant15=== undefined) {
            dataView(memory0).setInt8(arg3 + 20, 0, true);
          } else {
            const e = variant15;
            dataView(memory0).setInt8(arg3 + 20, 1, true);
            var ptr14 = utf8Encode(e, realloc0, memory0);
            var len14 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 28, len14, true);
            dataView(memory0).setUint32(arg3 + 24, ptr14, true);
          }
          break;
        }
        case 'HTTP-request-denied': {
          dataView(memory0).setInt8(arg3 + 8, 15, true);
          break;
        }
        case 'HTTP-request-length-required': {
          dataView(memory0).setInt8(arg3 + 8, 16, true);
          break;
        }
        case 'HTTP-request-body-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 17, true);
          var variant16 = e;
          if (variant16 === null || variant16=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant16;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setBigInt64(arg3 + 24, toUint64(e), true);
          }
          break;
        }
        case 'HTTP-request-method-invalid': {
          dataView(memory0).setInt8(arg3 + 8, 18, true);
          break;
        }
        case 'HTTP-request-URI-invalid': {
          dataView(memory0).setInt8(arg3 + 8, 19, true);
          break;
        }
        case 'HTTP-request-URI-too-long': {
          dataView(memory0).setInt8(arg3 + 8, 20, true);
          break;
        }
        case 'HTTP-request-header-section-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 21, true);
          var variant17 = e;
          if (variant17 === null || variant17=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant17;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-request-header-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 22, true);
          var variant22 = e;
          if (variant22 === null || variant22=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant22;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var {fieldName: v18_0, fieldSize: v18_1 } = e;
            var variant20 = v18_0;
            if (variant20 === null || variant20=== undefined) {
              dataView(memory0).setInt8(arg3 + 20, 0, true);
            } else {
              const e = variant20;
              dataView(memory0).setInt8(arg3 + 20, 1, true);
              var ptr19 = utf8Encode(e, realloc0, memory0);
              var len19 = utf8EncodedLen;
              dataView(memory0).setUint32(arg3 + 28, len19, true);
              dataView(memory0).setUint32(arg3 + 24, ptr19, true);
            }
            var variant21 = v18_1;
            if (variant21 === null || variant21=== undefined) {
              dataView(memory0).setInt8(arg3 + 32, 0, true);
            } else {
              const e = variant21;
              dataView(memory0).setInt8(arg3 + 32, 1, true);
              dataView(memory0).setInt32(arg3 + 36, toUint32(e), true);
            }
          }
          break;
        }
        case 'HTTP-request-trailer-section-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 23, true);
          var variant23 = e;
          if (variant23 === null || variant23=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant23;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-request-trailer-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 24, true);
          var {fieldName: v24_0, fieldSize: v24_1 } = e;
          var variant26 = v24_0;
          if (variant26 === null || variant26=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant26;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr25 = utf8Encode(e, realloc0, memory0);
            var len25 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len25, true);
            dataView(memory0).setUint32(arg3 + 20, ptr25, true);
          }
          var variant27 = v24_1;
          if (variant27 === null || variant27=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant27;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt32(arg3 + 32, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-incomplete': {
          dataView(memory0).setInt8(arg3 + 8, 25, true);
          break;
        }
        case 'HTTP-response-header-section-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 26, true);
          var variant28 = e;
          if (variant28 === null || variant28=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant28;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-header-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 27, true);
          var {fieldName: v29_0, fieldSize: v29_1 } = e;
          var variant31 = v29_0;
          if (variant31 === null || variant31=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant31;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr30 = utf8Encode(e, realloc0, memory0);
            var len30 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len30, true);
            dataView(memory0).setUint32(arg3 + 20, ptr30, true);
          }
          var variant32 = v29_1;
          if (variant32 === null || variant32=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant32;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt32(arg3 + 32, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-body-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 28, true);
          var variant33 = e;
          if (variant33 === null || variant33=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant33;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setBigInt64(arg3 + 24, toUint64(e), true);
          }
          break;
        }
        case 'HTTP-response-trailer-section-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 29, true);
          var variant34 = e;
          if (variant34 === null || variant34=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant34;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            dataView(memory0).setInt32(arg3 + 20, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-trailer-size': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 30, true);
          var {fieldName: v35_0, fieldSize: v35_1 } = e;
          var variant37 = v35_0;
          if (variant37 === null || variant37=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant37;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr36 = utf8Encode(e, realloc0, memory0);
            var len36 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len36, true);
            dataView(memory0).setUint32(arg3 + 20, ptr36, true);
          }
          var variant38 = v35_1;
          if (variant38 === null || variant38=== undefined) {
            dataView(memory0).setInt8(arg3 + 28, 0, true);
          } else {
            const e = variant38;
            dataView(memory0).setInt8(arg3 + 28, 1, true);
            dataView(memory0).setInt32(arg3 + 32, toUint32(e), true);
          }
          break;
        }
        case 'HTTP-response-transfer-coding': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 31, true);
          var variant40 = e;
          if (variant40 === null || variant40=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant40;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr39 = utf8Encode(e, realloc0, memory0);
            var len39 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len39, true);
            dataView(memory0).setUint32(arg3 + 20, ptr39, true);
          }
          break;
        }
        case 'HTTP-response-content-coding': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 32, true);
          var variant42 = e;
          if (variant42 === null || variant42=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant42;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr41 = utf8Encode(e, realloc0, memory0);
            var len41 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len41, true);
            dataView(memory0).setUint32(arg3 + 20, ptr41, true);
          }
          break;
        }
        case 'HTTP-response-timeout': {
          dataView(memory0).setInt8(arg3 + 8, 33, true);
          break;
        }
        case 'HTTP-upgrade-failed': {
          dataView(memory0).setInt8(arg3 + 8, 34, true);
          break;
        }
        case 'HTTP-protocol-error': {
          dataView(memory0).setInt8(arg3 + 8, 35, true);
          break;
        }
        case 'loop-detected': {
          dataView(memory0).setInt8(arg3 + 8, 36, true);
          break;
        }
        case 'configuration-error': {
          dataView(memory0).setInt8(arg3 + 8, 37, true);
          break;
        }
        case 'internal-error': {
          const e = variant45.val;
          dataView(memory0).setInt8(arg3 + 8, 38, true);
          var variant44 = e;
          if (variant44 === null || variant44=== undefined) {
            dataView(memory0).setInt8(arg3 + 16, 0, true);
          } else {
            const e = variant44;
            dataView(memory0).setInt8(arg3 + 16, 1, true);
            var ptr43 = utf8Encode(e, realloc0, memory0);
            var len43 = utf8EncodedLen;
            dataView(memory0).setUint32(arg3 + 24, len43, true);
            dataView(memory0).setUint32(arg3 + 20, ptr43, true);
          }
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant45.tag)}\` (received \`${variant45}\`) specified for \`ErrorCode\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:http/outgoing-handler@0.2.3", function="handle"][Instruction::Return]', {
    funcName: 'handle',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline61(arg0) {
  _debugLog('[iface="wasi:clocks/wall-clock@0.2.3", function="resolution"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'resolution');
  const ret = resolution$1();
  _debugLog('[iface="wasi:clocks/wall-clock@0.2.3", function="resolution"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var {seconds: v0_0, nanoseconds: v0_1 } = ret;
  dataView(memory0).setBigInt64(arg0 + 0, toUint64(v0_0), true);
  dataView(memory0).setInt32(arg0 + 8, toUint32(v0_1), true);
  _debugLog('[iface="wasi:clocks/wall-clock@0.2.3", function="resolution"][Instruction::Return]', {
    funcName: 'resolution',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline62(arg0) {
  _debugLog('[iface="wasi:clocks/wall-clock@0.2.3", function="now"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'now');
  const ret = now$1();
  _debugLog('[iface="wasi:clocks/wall-clock@0.2.3", function="now"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var {seconds: v0_0, nanoseconds: v0_1 } = ret;
  dataView(memory0).setBigInt64(arg0 + 0, toUint64(v0_0), true);
  dataView(memory0).setInt32(arg0 + 8, toUint32(v0_1), true);
  _debugLog('[iface="wasi:clocks/wall-clock@0.2.3", function="now"][Instruction::Return]', {
    funcName: 'now',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable6 = [T_FLAG, 0];
const captureTable6= new Map();
let captureCnt6 = 0;
handleTables[6] = handleTable6;

function trampoline63(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable6[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable6.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Descriptor.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.get-flags"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]descriptor.get-flags');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.getFlags()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.get-flags"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      let flags3 = 0;
      if (typeof e === 'object' && e !== null) {
        flags3 = Boolean(e.read) << 0 | Boolean(e.write) << 1 | Boolean(e.fileIntegritySync) << 2 | Boolean(e.dataIntegritySync) << 3 | Boolean(e.requestedWriteSync) << 4 | Boolean(e.mutateDirectory) << 5;
      } else if (e !== null && e!== undefined) {
        throw new TypeError('only an object, undefined or null can be converted to flags');
      }
      dataView(memory0).setInt8(arg1 + 1, flags3, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      var val4 = e;
      let enum4;
      switch (val4) {
        case 'access': {
          enum4 = 0;
          break;
        }
        case 'would-block': {
          enum4 = 1;
          break;
        }
        case 'already': {
          enum4 = 2;
          break;
        }
        case 'bad-descriptor': {
          enum4 = 3;
          break;
        }
        case 'busy': {
          enum4 = 4;
          break;
        }
        case 'deadlock': {
          enum4 = 5;
          break;
        }
        case 'quota': {
          enum4 = 6;
          break;
        }
        case 'exist': {
          enum4 = 7;
          break;
        }
        case 'file-too-large': {
          enum4 = 8;
          break;
        }
        case 'illegal-byte-sequence': {
          enum4 = 9;
          break;
        }
        case 'in-progress': {
          enum4 = 10;
          break;
        }
        case 'interrupted': {
          enum4 = 11;
          break;
        }
        case 'invalid': {
          enum4 = 12;
          break;
        }
        case 'io': {
          enum4 = 13;
          break;
        }
        case 'is-directory': {
          enum4 = 14;
          break;
        }
        case 'loop': {
          enum4 = 15;
          break;
        }
        case 'too-many-links': {
          enum4 = 16;
          break;
        }
        case 'message-size': {
          enum4 = 17;
          break;
        }
        case 'name-too-long': {
          enum4 = 18;
          break;
        }
        case 'no-device': {
          enum4 = 19;
          break;
        }
        case 'no-entry': {
          enum4 = 20;
          break;
        }
        case 'no-lock': {
          enum4 = 21;
          break;
        }
        case 'insufficient-memory': {
          enum4 = 22;
          break;
        }
        case 'insufficient-space': {
          enum4 = 23;
          break;
        }
        case 'not-directory': {
          enum4 = 24;
          break;
        }
        case 'not-empty': {
          enum4 = 25;
          break;
        }
        case 'not-recoverable': {
          enum4 = 26;
          break;
        }
        case 'unsupported': {
          enum4 = 27;
          break;
        }
        case 'no-tty': {
          enum4 = 28;
          break;
        }
        case 'no-such-device': {
          enum4 = 29;
          break;
        }
        case 'overflow': {
          enum4 = 30;
          break;
        }
        case 'not-permitted': {
          enum4 = 31;
          break;
        }
        case 'pipe': {
          enum4 = 32;
          break;
        }
        case 'read-only': {
          enum4 = 33;
          break;
        }
        case 'invalid-seek': {
          enum4 = 34;
          break;
        }
        case 'text-file-busy': {
          enum4 = 35;
          break;
        }
        case 'cross-device': {
          enum4 = 36;
          break;
        }
        default: {
          if ((e) instanceof Error) {
            console.error(e);
          }
          
          throw new TypeError(`"${val4}" is not one of the cases of error-code`);
        }
      }
      dataView(memory0).setInt8(arg1 + 1, enum4, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.get-flags"][Instruction::Return]', {
    funcName: '[method]descriptor.get-flags',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline64(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable6[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable6.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Descriptor.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.get-type"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]descriptor.get-type');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.getType()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.get-type"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      var val3 = e;
      let enum3;
      switch (val3) {
        case 'unknown': {
          enum3 = 0;
          break;
        }
        case 'block-device': {
          enum3 = 1;
          break;
        }
        case 'character-device': {
          enum3 = 2;
          break;
        }
        case 'directory': {
          enum3 = 3;
          break;
        }
        case 'fifo': {
          enum3 = 4;
          break;
        }
        case 'symbolic-link': {
          enum3 = 5;
          break;
        }
        case 'regular-file': {
          enum3 = 6;
          break;
        }
        case 'socket': {
          enum3 = 7;
          break;
        }
        default: {
          if ((e) instanceof Error) {
            console.error(e);
          }
          
          throw new TypeError(`"${val3}" is not one of the cases of descriptor-type`);
        }
      }
      dataView(memory0).setInt8(arg1 + 1, enum3, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      var val4 = e;
      let enum4;
      switch (val4) {
        case 'access': {
          enum4 = 0;
          break;
        }
        case 'would-block': {
          enum4 = 1;
          break;
        }
        case 'already': {
          enum4 = 2;
          break;
        }
        case 'bad-descriptor': {
          enum4 = 3;
          break;
        }
        case 'busy': {
          enum4 = 4;
          break;
        }
        case 'deadlock': {
          enum4 = 5;
          break;
        }
        case 'quota': {
          enum4 = 6;
          break;
        }
        case 'exist': {
          enum4 = 7;
          break;
        }
        case 'file-too-large': {
          enum4 = 8;
          break;
        }
        case 'illegal-byte-sequence': {
          enum4 = 9;
          break;
        }
        case 'in-progress': {
          enum4 = 10;
          break;
        }
        case 'interrupted': {
          enum4 = 11;
          break;
        }
        case 'invalid': {
          enum4 = 12;
          break;
        }
        case 'io': {
          enum4 = 13;
          break;
        }
        case 'is-directory': {
          enum4 = 14;
          break;
        }
        case 'loop': {
          enum4 = 15;
          break;
        }
        case 'too-many-links': {
          enum4 = 16;
          break;
        }
        case 'message-size': {
          enum4 = 17;
          break;
        }
        case 'name-too-long': {
          enum4 = 18;
          break;
        }
        case 'no-device': {
          enum4 = 19;
          break;
        }
        case 'no-entry': {
          enum4 = 20;
          break;
        }
        case 'no-lock': {
          enum4 = 21;
          break;
        }
        case 'insufficient-memory': {
          enum4 = 22;
          break;
        }
        case 'insufficient-space': {
          enum4 = 23;
          break;
        }
        case 'not-directory': {
          enum4 = 24;
          break;
        }
        case 'not-empty': {
          enum4 = 25;
          break;
        }
        case 'not-recoverable': {
          enum4 = 26;
          break;
        }
        case 'unsupported': {
          enum4 = 27;
          break;
        }
        case 'no-tty': {
          enum4 = 28;
          break;
        }
        case 'no-such-device': {
          enum4 = 29;
          break;
        }
        case 'overflow': {
          enum4 = 30;
          break;
        }
        case 'not-permitted': {
          enum4 = 31;
          break;
        }
        case 'pipe': {
          enum4 = 32;
          break;
        }
        case 'read-only': {
          enum4 = 33;
          break;
        }
        case 'invalid-seek': {
          enum4 = 34;
          break;
        }
        case 'text-file-busy': {
          enum4 = 35;
          break;
        }
        case 'cross-device': {
          enum4 = 36;
          break;
        }
        default: {
          if ((e) instanceof Error) {
            console.error(e);
          }
          
          throw new TypeError(`"${val4}" is not one of the cases of error-code`);
        }
      }
      dataView(memory0).setInt8(arg1 + 1, enum4, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.get-type"][Instruction::Return]', {
    funcName: '[method]descriptor.get-type',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline65(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable0[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable0.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Error$1.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="filesystem-error-code"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'filesystem-error-code');
  const ret = filesystemErrorCode(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="filesystem-error-code"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant4 = ret;
  if (variant4 === null || variant4=== undefined) {
    dataView(memory0).setInt8(arg1 + 0, 0, true);
  } else {
    const e = variant4;
    dataView(memory0).setInt8(arg1 + 0, 1, true);
    var val3 = e;
    let enum3;
    switch (val3) {
      case 'access': {
        enum3 = 0;
        break;
      }
      case 'would-block': {
        enum3 = 1;
        break;
      }
      case 'already': {
        enum3 = 2;
        break;
      }
      case 'bad-descriptor': {
        enum3 = 3;
        break;
      }
      case 'busy': {
        enum3 = 4;
        break;
      }
      case 'deadlock': {
        enum3 = 5;
        break;
      }
      case 'quota': {
        enum3 = 6;
        break;
      }
      case 'exist': {
        enum3 = 7;
        break;
      }
      case 'file-too-large': {
        enum3 = 8;
        break;
      }
      case 'illegal-byte-sequence': {
        enum3 = 9;
        break;
      }
      case 'in-progress': {
        enum3 = 10;
        break;
      }
      case 'interrupted': {
        enum3 = 11;
        break;
      }
      case 'invalid': {
        enum3 = 12;
        break;
      }
      case 'io': {
        enum3 = 13;
        break;
      }
      case 'is-directory': {
        enum3 = 14;
        break;
      }
      case 'loop': {
        enum3 = 15;
        break;
      }
      case 'too-many-links': {
        enum3 = 16;
        break;
      }
      case 'message-size': {
        enum3 = 17;
        break;
      }
      case 'name-too-long': {
        enum3 = 18;
        break;
      }
      case 'no-device': {
        enum3 = 19;
        break;
      }
      case 'no-entry': {
        enum3 = 20;
        break;
      }
      case 'no-lock': {
        enum3 = 21;
        break;
      }
      case 'insufficient-memory': {
        enum3 = 22;
        break;
      }
      case 'insufficient-space': {
        enum3 = 23;
        break;
      }
      case 'not-directory': {
        enum3 = 24;
        break;
      }
      case 'not-empty': {
        enum3 = 25;
        break;
      }
      case 'not-recoverable': {
        enum3 = 26;
        break;
      }
      case 'unsupported': {
        enum3 = 27;
        break;
      }
      case 'no-tty': {
        enum3 = 28;
        break;
      }
      case 'no-such-device': {
        enum3 = 29;
        break;
      }
      case 'overflow': {
        enum3 = 30;
        break;
      }
      case 'not-permitted': {
        enum3 = 31;
        break;
      }
      case 'pipe': {
        enum3 = 32;
        break;
      }
      case 'read-only': {
        enum3 = 33;
        break;
      }
      case 'invalid-seek': {
        enum3 = 34;
        break;
      }
      case 'text-file-busy': {
        enum3 = 35;
        break;
      }
      case 'cross-device': {
        enum3 = 36;
        break;
      }
      default: {
        if ((e) instanceof Error) {
          console.error(e);
        }
        
        throw new TypeError(`"${val3}" is not one of the cases of error-code`);
      }
    }
    dataView(memory0).setInt8(arg1 + 1, enum3, true);
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="filesystem-error-code"][Instruction::Return]', {
    funcName: 'filesystem-error-code',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline66(arg0, arg1, arg2) {
  var handle1 = arg0;
  var rep2 = handleTable6[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable6.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Descriptor.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.write-via-stream"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]descriptor.write-via-stream');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.writeViaStream(BigInt.asUintN(64, arg1))};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.write-via-stream"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg2 + 0, 0, true);
      if (!(e instanceof OutputStream)) {
        throw new TypeError('Resource error: Not a valid "OutputStream" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt3;
        captureTable3.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable3, rep);
      }
      dataView(memory0).setInt32(arg2 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg2 + 0, 1, true);
      var val4 = e;
      let enum4;
      switch (val4) {
        case 'access': {
          enum4 = 0;
          break;
        }
        case 'would-block': {
          enum4 = 1;
          break;
        }
        case 'already': {
          enum4 = 2;
          break;
        }
        case 'bad-descriptor': {
          enum4 = 3;
          break;
        }
        case 'busy': {
          enum4 = 4;
          break;
        }
        case 'deadlock': {
          enum4 = 5;
          break;
        }
        case 'quota': {
          enum4 = 6;
          break;
        }
        case 'exist': {
          enum4 = 7;
          break;
        }
        case 'file-too-large': {
          enum4 = 8;
          break;
        }
        case 'illegal-byte-sequence': {
          enum4 = 9;
          break;
        }
        case 'in-progress': {
          enum4 = 10;
          break;
        }
        case 'interrupted': {
          enum4 = 11;
          break;
        }
        case 'invalid': {
          enum4 = 12;
          break;
        }
        case 'io': {
          enum4 = 13;
          break;
        }
        case 'is-directory': {
          enum4 = 14;
          break;
        }
        case 'loop': {
          enum4 = 15;
          break;
        }
        case 'too-many-links': {
          enum4 = 16;
          break;
        }
        case 'message-size': {
          enum4 = 17;
          break;
        }
        case 'name-too-long': {
          enum4 = 18;
          break;
        }
        case 'no-device': {
          enum4 = 19;
          break;
        }
        case 'no-entry': {
          enum4 = 20;
          break;
        }
        case 'no-lock': {
          enum4 = 21;
          break;
        }
        case 'insufficient-memory': {
          enum4 = 22;
          break;
        }
        case 'insufficient-space': {
          enum4 = 23;
          break;
        }
        case 'not-directory': {
          enum4 = 24;
          break;
        }
        case 'not-empty': {
          enum4 = 25;
          break;
        }
        case 'not-recoverable': {
          enum4 = 26;
          break;
        }
        case 'unsupported': {
          enum4 = 27;
          break;
        }
        case 'no-tty': {
          enum4 = 28;
          break;
        }
        case 'no-such-device': {
          enum4 = 29;
          break;
        }
        case 'overflow': {
          enum4 = 30;
          break;
        }
        case 'not-permitted': {
          enum4 = 31;
          break;
        }
        case 'pipe': {
          enum4 = 32;
          break;
        }
        case 'read-only': {
          enum4 = 33;
          break;
        }
        case 'invalid-seek': {
          enum4 = 34;
          break;
        }
        case 'text-file-busy': {
          enum4 = 35;
          break;
        }
        case 'cross-device': {
          enum4 = 36;
          break;
        }
        default: {
          if ((e) instanceof Error) {
            console.error(e);
          }
          
          throw new TypeError(`"${val4}" is not one of the cases of error-code`);
        }
      }
      dataView(memory0).setInt8(arg2 + 4, enum4, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.write-via-stream"][Instruction::Return]', {
    funcName: '[method]descriptor.write-via-stream',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline67(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable6[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable6.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Descriptor.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.append-via-stream"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]descriptor.append-via-stream');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.appendViaStream()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.append-via-stream"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant5 = ret;
  switch (variant5.tag) {
    case 'ok': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      if (!(e instanceof OutputStream)) {
        throw new TypeError('Resource error: Not a valid "OutputStream" resource.');
      }
      var handle3 = e[symbolRscHandle];
      if (!handle3) {
        const rep = e[symbolRscRep] || ++captureCnt3;
        captureTable3.set(rep, e);
        handle3 = rscTableCreateOwn(handleTable3, rep);
      }
      dataView(memory0).setInt32(arg1 + 4, handle3, true);
      break;
    }
    case 'err': {
      const e = variant5.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      var val4 = e;
      let enum4;
      switch (val4) {
        case 'access': {
          enum4 = 0;
          break;
        }
        case 'would-block': {
          enum4 = 1;
          break;
        }
        case 'already': {
          enum4 = 2;
          break;
        }
        case 'bad-descriptor': {
          enum4 = 3;
          break;
        }
        case 'busy': {
          enum4 = 4;
          break;
        }
        case 'deadlock': {
          enum4 = 5;
          break;
        }
        case 'quota': {
          enum4 = 6;
          break;
        }
        case 'exist': {
          enum4 = 7;
          break;
        }
        case 'file-too-large': {
          enum4 = 8;
          break;
        }
        case 'illegal-byte-sequence': {
          enum4 = 9;
          break;
        }
        case 'in-progress': {
          enum4 = 10;
          break;
        }
        case 'interrupted': {
          enum4 = 11;
          break;
        }
        case 'invalid': {
          enum4 = 12;
          break;
        }
        case 'io': {
          enum4 = 13;
          break;
        }
        case 'is-directory': {
          enum4 = 14;
          break;
        }
        case 'loop': {
          enum4 = 15;
          break;
        }
        case 'too-many-links': {
          enum4 = 16;
          break;
        }
        case 'message-size': {
          enum4 = 17;
          break;
        }
        case 'name-too-long': {
          enum4 = 18;
          break;
        }
        case 'no-device': {
          enum4 = 19;
          break;
        }
        case 'no-entry': {
          enum4 = 20;
          break;
        }
        case 'no-lock': {
          enum4 = 21;
          break;
        }
        case 'insufficient-memory': {
          enum4 = 22;
          break;
        }
        case 'insufficient-space': {
          enum4 = 23;
          break;
        }
        case 'not-directory': {
          enum4 = 24;
          break;
        }
        case 'not-empty': {
          enum4 = 25;
          break;
        }
        case 'not-recoverable': {
          enum4 = 26;
          break;
        }
        case 'unsupported': {
          enum4 = 27;
          break;
        }
        case 'no-tty': {
          enum4 = 28;
          break;
        }
        case 'no-such-device': {
          enum4 = 29;
          break;
        }
        case 'overflow': {
          enum4 = 30;
          break;
        }
        case 'not-permitted': {
          enum4 = 31;
          break;
        }
        case 'pipe': {
          enum4 = 32;
          break;
        }
        case 'read-only': {
          enum4 = 33;
          break;
        }
        case 'invalid-seek': {
          enum4 = 34;
          break;
        }
        case 'text-file-busy': {
          enum4 = 35;
          break;
        }
        case 'cross-device': {
          enum4 = 36;
          break;
        }
        default: {
          if ((e) instanceof Error) {
            console.error(e);
          }
          
          throw new TypeError(`"${val4}" is not one of the cases of error-code`);
        }
      }
      dataView(memory0).setInt8(arg1 + 4, enum4, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.append-via-stream"][Instruction::Return]', {
    funcName: '[method]descriptor.append-via-stream',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline68(arg0, arg1) {
  var handle1 = arg0;
  var rep2 = handleTable6[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable6.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(Descriptor.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.stat"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]descriptor.stat');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.stat()};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.stat"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant12 = ret;
  switch (variant12.tag) {
    case 'ok': {
      const e = variant12.val;
      dataView(memory0).setInt8(arg1 + 0, 0, true);
      var {type: v3_0, linkCount: v3_1, size: v3_2, dataAccessTimestamp: v3_3, dataModificationTimestamp: v3_4, statusChangeTimestamp: v3_5 } = e;
      var val4 = v3_0;
      let enum4;
      switch (val4) {
        case 'unknown': {
          enum4 = 0;
          break;
        }
        case 'block-device': {
          enum4 = 1;
          break;
        }
        case 'character-device': {
          enum4 = 2;
          break;
        }
        case 'directory': {
          enum4 = 3;
          break;
        }
        case 'fifo': {
          enum4 = 4;
          break;
        }
        case 'symbolic-link': {
          enum4 = 5;
          break;
        }
        case 'regular-file': {
          enum4 = 6;
          break;
        }
        case 'socket': {
          enum4 = 7;
          break;
        }
        default: {
          if ((v3_0) instanceof Error) {
            console.error(v3_0);
          }
          
          throw new TypeError(`"${val4}" is not one of the cases of descriptor-type`);
        }
      }
      dataView(memory0).setInt8(arg1 + 8, enum4, true);
      dataView(memory0).setBigInt64(arg1 + 16, toUint64(v3_1), true);
      dataView(memory0).setBigInt64(arg1 + 24, toUint64(v3_2), true);
      var variant6 = v3_3;
      if (variant6 === null || variant6=== undefined) {
        dataView(memory0).setInt8(arg1 + 32, 0, true);
      } else {
        const e = variant6;
        dataView(memory0).setInt8(arg1 + 32, 1, true);
        var {seconds: v5_0, nanoseconds: v5_1 } = e;
        dataView(memory0).setBigInt64(arg1 + 40, toUint64(v5_0), true);
        dataView(memory0).setInt32(arg1 + 48, toUint32(v5_1), true);
      }
      var variant8 = v3_4;
      if (variant8 === null || variant8=== undefined) {
        dataView(memory0).setInt8(arg1 + 56, 0, true);
      } else {
        const e = variant8;
        dataView(memory0).setInt8(arg1 + 56, 1, true);
        var {seconds: v7_0, nanoseconds: v7_1 } = e;
        dataView(memory0).setBigInt64(arg1 + 64, toUint64(v7_0), true);
        dataView(memory0).setInt32(arg1 + 72, toUint32(v7_1), true);
      }
      var variant10 = v3_5;
      if (variant10 === null || variant10=== undefined) {
        dataView(memory0).setInt8(arg1 + 80, 0, true);
      } else {
        const e = variant10;
        dataView(memory0).setInt8(arg1 + 80, 1, true);
        var {seconds: v9_0, nanoseconds: v9_1 } = e;
        dataView(memory0).setBigInt64(arg1 + 88, toUint64(v9_0), true);
        dataView(memory0).setInt32(arg1 + 96, toUint32(v9_1), true);
      }
      break;
    }
    case 'err': {
      const e = variant12.val;
      dataView(memory0).setInt8(arg1 + 0, 1, true);
      var val11 = e;
      let enum11;
      switch (val11) {
        case 'access': {
          enum11 = 0;
          break;
        }
        case 'would-block': {
          enum11 = 1;
          break;
        }
        case 'already': {
          enum11 = 2;
          break;
        }
        case 'bad-descriptor': {
          enum11 = 3;
          break;
        }
        case 'busy': {
          enum11 = 4;
          break;
        }
        case 'deadlock': {
          enum11 = 5;
          break;
        }
        case 'quota': {
          enum11 = 6;
          break;
        }
        case 'exist': {
          enum11 = 7;
          break;
        }
        case 'file-too-large': {
          enum11 = 8;
          break;
        }
        case 'illegal-byte-sequence': {
          enum11 = 9;
          break;
        }
        case 'in-progress': {
          enum11 = 10;
          break;
        }
        case 'interrupted': {
          enum11 = 11;
          break;
        }
        case 'invalid': {
          enum11 = 12;
          break;
        }
        case 'io': {
          enum11 = 13;
          break;
        }
        case 'is-directory': {
          enum11 = 14;
          break;
        }
        case 'loop': {
          enum11 = 15;
          break;
        }
        case 'too-many-links': {
          enum11 = 16;
          break;
        }
        case 'message-size': {
          enum11 = 17;
          break;
        }
        case 'name-too-long': {
          enum11 = 18;
          break;
        }
        case 'no-device': {
          enum11 = 19;
          break;
        }
        case 'no-entry': {
          enum11 = 20;
          break;
        }
        case 'no-lock': {
          enum11 = 21;
          break;
        }
        case 'insufficient-memory': {
          enum11 = 22;
          break;
        }
        case 'insufficient-space': {
          enum11 = 23;
          break;
        }
        case 'not-directory': {
          enum11 = 24;
          break;
        }
        case 'not-empty': {
          enum11 = 25;
          break;
        }
        case 'not-recoverable': {
          enum11 = 26;
          break;
        }
        case 'unsupported': {
          enum11 = 27;
          break;
        }
        case 'no-tty': {
          enum11 = 28;
          break;
        }
        case 'no-such-device': {
          enum11 = 29;
          break;
        }
        case 'overflow': {
          enum11 = 30;
          break;
        }
        case 'not-permitted': {
          enum11 = 31;
          break;
        }
        case 'pipe': {
          enum11 = 32;
          break;
        }
        case 'read-only': {
          enum11 = 33;
          break;
        }
        case 'invalid-seek': {
          enum11 = 34;
          break;
        }
        case 'text-file-busy': {
          enum11 = 35;
          break;
        }
        case 'cross-device': {
          enum11 = 36;
          break;
        }
        default: {
          if ((e) instanceof Error) {
            console.error(e);
          }
          
          throw new TypeError(`"${val11}" is not one of the cases of error-code`);
        }
      }
      dataView(memory0).setInt8(arg1 + 8, enum11, true);
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:filesystem/types@0.2.3", function="[method]descriptor.stat"][Instruction::Return]', {
    funcName: '[method]descriptor.stat',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline69(arg0, arg1, arg2, arg3) {
  var handle1 = arg0;
  var rep2 = handleTable3[(handle1 << 1) + 1] & ~T_FLAG;
  var rsc0 = captureTable3.get(rep2);
  if (!rsc0) {
    rsc0 = Object.create(OutputStream.prototype);
    Object.defineProperty(rsc0, symbolRscHandle, { writable: true, value: handle1});
    Object.defineProperty(rsc0, symbolRscRep, { writable: true, value: rep2});
  }
  curResourceBorrows.push(rsc0);
  var ptr3 = arg1;
  var len3 = arg2;
  var result3 = new Uint8Array(memory0.buffer.slice(ptr3, ptr3 + len3 * 1));
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.blocking-write-and-flush"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, '[method]output-stream.blocking-write-and-flush');
  let ret;
  try {
    ret = { tag: 'ok', val: rsc0.blockingWriteAndFlush(result3)};
  } catch (e) {
    ret = { tag: 'err', val: getErrorPayload(e) };
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.blocking-write-and-flush"] [Instruction::CallInterface] (sync, @ post-call)');
  for (const rsc of curResourceBorrows) {
    rsc[symbolRscHandle] = undefined;
  }
  curResourceBorrows = [];
  endCurrentTask(0);
  var variant6 = ret;
  switch (variant6.tag) {
    case 'ok': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg3 + 0, 0, true);
      break;
    }
    case 'err': {
      const e = variant6.val;
      dataView(memory0).setInt8(arg3 + 0, 1, true);
      var variant5 = e;
      switch (variant5.tag) {
        case 'last-operation-failed': {
          const e = variant5.val;
          dataView(memory0).setInt8(arg3 + 4, 0, true);
          if (!(e instanceof Error$1)) {
            throw new TypeError('Resource error: Not a valid "Error" resource.');
          }
          var handle4 = e[symbolRscHandle];
          if (!handle4) {
            const rep = e[symbolRscRep] || ++captureCnt0;
            captureTable0.set(rep, e);
            handle4 = rscTableCreateOwn(handleTable0, rep);
          }
          dataView(memory0).setInt32(arg3 + 8, handle4, true);
          break;
        }
        case 'closed': {
          dataView(memory0).setInt8(arg3 + 4, 1, true);
          break;
        }
        default: {
          throw new TypeError(`invalid variant tag value \`${JSON.stringify(variant5.tag)}\` (received \`${variant5}\`) specified for \`StreamError\``);
        }
      }
      break;
    }
    default: {
      throw new TypeError('invalid variant specified for result');
    }
  }
  _debugLog('[iface="wasi:io/streams@0.2.3", function="[method]output-stream.blocking-write-and-flush"][Instruction::Return]', {
    funcName: '[method]output-stream.blocking-write-and-flush',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline70(arg0) {
  _debugLog('[iface="wasi:filesystem/preopens@0.2.3", function="get-directories"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-directories');
  const ret = getDirectories();
  _debugLog('[iface="wasi:filesystem/preopens@0.2.3", function="get-directories"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var vec3 = ret;
  var len3 = vec3.length;
  var result3 = realloc1(0, 0, 4, len3 * 12);
  for (let i = 0; i < vec3.length; i++) {
    const e = vec3[i];
    const base = result3 + i * 12;var [tuple0_0, tuple0_1] = e;
    if (!(tuple0_0 instanceof Descriptor)) {
      throw new TypeError('Resource error: Not a valid "Descriptor" resource.');
    }
    var handle1 = tuple0_0[symbolRscHandle];
    if (!handle1) {
      const rep = tuple0_0[symbolRscRep] || ++captureCnt6;
      captureTable6.set(rep, tuple0_0);
      handle1 = rscTableCreateOwn(handleTable6, rep);
    }
    dataView(memory0).setInt32(base + 0, handle1, true);
    var ptr2 = utf8Encode(tuple0_1, realloc1, memory0);
    var len2 = utf8EncodedLen;
    dataView(memory0).setUint32(base + 8, len2, true);
    dataView(memory0).setUint32(base + 4, ptr2, true);
  }
  dataView(memory0).setUint32(arg0 + 4, len3, true);
  dataView(memory0).setUint32(arg0 + 0, result3, true);
  _debugLog('[iface="wasi:filesystem/preopens@0.2.3", function="get-directories"][Instruction::Return]', {
    funcName: 'get-directories',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable4 = [T_FLAG, 0];
const captureTable4= new Map();
let captureCnt4 = 0;
handleTables[4] = handleTable4;

function trampoline71(arg0) {
  _debugLog('[iface="wasi:cli/terminal-stdin@0.2.3", function="get-terminal-stdin"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-terminal-stdin');
  const ret = getTerminalStdin();
  _debugLog('[iface="wasi:cli/terminal-stdin@0.2.3", function="get-terminal-stdin"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var variant1 = ret;
  if (variant1 === null || variant1=== undefined) {
    dataView(memory0).setInt8(arg0 + 0, 0, true);
  } else {
    const e = variant1;
    dataView(memory0).setInt8(arg0 + 0, 1, true);
    if (!(e instanceof TerminalInput)) {
      throw new TypeError('Resource error: Not a valid "TerminalInput" resource.');
    }
    var handle0 = e[symbolRscHandle];
    if (!handle0) {
      const rep = e[symbolRscRep] || ++captureCnt4;
      captureTable4.set(rep, e);
      handle0 = rscTableCreateOwn(handleTable4, rep);
    }
    dataView(memory0).setInt32(arg0 + 4, handle0, true);
  }
  _debugLog('[iface="wasi:cli/terminal-stdin@0.2.3", function="get-terminal-stdin"][Instruction::Return]', {
    funcName: 'get-terminal-stdin',
    paramCount: 0,
    postReturn: false
  });
}

const handleTable5 = [T_FLAG, 0];
const captureTable5= new Map();
let captureCnt5 = 0;
handleTables[5] = handleTable5;

function trampoline72(arg0) {
  _debugLog('[iface="wasi:cli/terminal-stdout@0.2.3", function="get-terminal-stdout"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-terminal-stdout');
  const ret = getTerminalStdout();
  _debugLog('[iface="wasi:cli/terminal-stdout@0.2.3", function="get-terminal-stdout"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var variant1 = ret;
  if (variant1 === null || variant1=== undefined) {
    dataView(memory0).setInt8(arg0 + 0, 0, true);
  } else {
    const e = variant1;
    dataView(memory0).setInt8(arg0 + 0, 1, true);
    if (!(e instanceof TerminalOutput)) {
      throw new TypeError('Resource error: Not a valid "TerminalOutput" resource.');
    }
    var handle0 = e[symbolRscHandle];
    if (!handle0) {
      const rep = e[symbolRscRep] || ++captureCnt5;
      captureTable5.set(rep, e);
      handle0 = rscTableCreateOwn(handleTable5, rep);
    }
    dataView(memory0).setInt32(arg0 + 4, handle0, true);
  }
  _debugLog('[iface="wasi:cli/terminal-stdout@0.2.3", function="get-terminal-stdout"][Instruction::Return]', {
    funcName: 'get-terminal-stdout',
    paramCount: 0,
    postReturn: false
  });
}


function trampoline73(arg0) {
  _debugLog('[iface="wasi:cli/terminal-stderr@0.2.3", function="get-terminal-stderr"] [Instruction::CallInterface] (async? sync, @ enter)');
  const _interface_call_currentTaskID = startCurrentTask(0, false, 'get-terminal-stderr');
  const ret = getTerminalStderr();
  _debugLog('[iface="wasi:cli/terminal-stderr@0.2.3", function="get-terminal-stderr"] [Instruction::CallInterface] (sync, @ post-call)');
  endCurrentTask(0);
  var variant1 = ret;
  if (variant1 === null || variant1=== undefined) {
    dataView(memory0).setInt8(arg0 + 0, 0, true);
  } else {
    const e = variant1;
    dataView(memory0).setInt8(arg0 + 0, 1, true);
    if (!(e instanceof TerminalOutput)) {
      throw new TypeError('Resource error: Not a valid "TerminalOutput" resource.');
    }
    var handle0 = e[symbolRscHandle];
    if (!handle0) {
      const rep = e[symbolRscRep] || ++captureCnt5;
      captureTable5.set(rep, e);
      handle0 = rscTableCreateOwn(handleTable5, rep);
    }
    dataView(memory0).setInt32(arg0 + 4, handle0, true);
  }
  _debugLog('[iface="wasi:cli/terminal-stderr@0.2.3", function="get-terminal-stderr"][Instruction::Return]', {
    funcName: 'get-terminal-stderr',
    paramCount: 0,
    postReturn: false
  });
}

let exports3;
let postReturn0;
let postReturn1;
function trampoline0(handle) {
  const handleEntry = rscTableRemove(handleTable1, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable1.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable1.delete(handleEntry.rep);
    } else if (Pollable[symbolCabiDispose]) {
      Pollable[symbolCabiDispose](handleEntry.rep);
    }
  }
}
function trampoline1(handle) {
  const handleEntry = rscTableRemove(handleTable2, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable2.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable2.delete(handleEntry.rep);
    } else if (InputStream[symbolCabiDispose]) {
      InputStream[symbolCabiDispose](handleEntry.rep);
    }
  }
}
function trampoline2(handle) {
  const handleEntry = rscTableRemove(handleTable3, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable3.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable3.delete(handleEntry.rep);
    } else if (OutputStream[symbolCabiDispose]) {
      OutputStream[symbolCabiDispose](handleEntry.rep);
    }
  }
}
function trampoline22(handle) {
  const handleEntry = rscTableRemove(handleTable0, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable0.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable0.delete(handleEntry.rep);
    } else if (Error$1[symbolCabiDispose]) {
      Error$1[symbolCabiDispose](handleEntry.rep);
    }
  }
}
function trampoline23(handle) {
  const handleEntry = rscTableRemove(handleTable6, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable6.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable6.delete(handleEntry.rep);
    } else if (Descriptor[symbolCabiDispose]) {
      Descriptor[symbolCabiDispose](handleEntry.rep);
    }
  }
}
function trampoline25(handle) {
  const handleEntry = rscTableRemove(handleTable4, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable4.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable4.delete(handleEntry.rep);
    } else if (TerminalInput[symbolCabiDispose]) {
      TerminalInput[symbolCabiDispose](handleEntry.rep);
    }
  }
}
function trampoline26(handle) {
  const handleEntry = rscTableRemove(handleTable5, handle);
  if (handleEntry.own) {
    
    const rsc = captureTable5.get(handleEntry.rep);
    if (rsc) {
      if (rsc[symbolDispose]) rsc[symbolDispose]();
      captureTable5.delete(handleEntry.rep);
    } else if (TerminalOutput[symbolCabiDispose]) {
      TerminalOutput[symbolCabiDispose](handleEntry.rep);
    }
  }
}
let toolHandler019HandleListTools;

function handleListTools(arg0) {
  var {cursor: v0_0, progressToken: v0_1, meta: v0_2 } = arg0;
  var variant2 = v0_0;
  let variant2_0;
  let variant2_1;
  let variant2_2;
  if (variant2 === null || variant2=== undefined) {
    variant2_0 = 0;
    variant2_1 = 0;
    variant2_2 = 0;
  } else {
    const e = variant2;
    var ptr1 = utf8Encode(e, realloc0, memory0);
    var len1 = utf8EncodedLen;
    variant2_0 = 1;
    variant2_1 = ptr1;
    variant2_2 = len1;
  }
  var variant4 = v0_1;
  let variant4_0;
  let variant4_1;
  let variant4_2;
  if (variant4 === null || variant4=== undefined) {
    variant4_0 = 0;
    variant4_1 = 0;
    variant4_2 = 0;
  } else {
    const e = variant4;
    var ptr3 = utf8Encode(e, realloc0, memory0);
    var len3 = utf8EncodedLen;
    variant4_0 = 1;
    variant4_1 = ptr3;
    variant4_2 = len3;
  }
  var variant9 = v0_2;
  let variant9_0;
  let variant9_1;
  let variant9_2;
  if (variant9 === null || variant9=== undefined) {
    variant9_0 = 0;
    variant9_1 = 0;
    variant9_2 = 0;
  } else {
    const e = variant9;
    var vec8 = e;
    var len8 = vec8.length;
    var result8 = realloc0(0, 0, 4, len8 * 16);
    for (let i = 0; i < vec8.length; i++) {
      const e = vec8[i];
      const base = result8 + i * 16;var [tuple5_0, tuple5_1] = e;
      var ptr6 = utf8Encode(tuple5_0, realloc0, memory0);
      var len6 = utf8EncodedLen;
      dataView(memory0).setUint32(base + 4, len6, true);
      dataView(memory0).setUint32(base + 0, ptr6, true);
      var ptr7 = utf8Encode(tuple5_1, realloc0, memory0);
      var len7 = utf8EncodedLen;
      dataView(memory0).setUint32(base + 12, len7, true);
      dataView(memory0).setUint32(base + 8, ptr7, true);
    }
    variant9_0 = 1;
    variant9_1 = result8;
    variant9_2 = len8;
  }
  _debugLog('[iface="fastertools:mcp/tool-handler@0.1.9", function="handle-list-tools"] [Instruction::CallWasm] (async? false, @ enter)');
  const _wasm_call_currentTaskID = startCurrentTask(0, false, 'toolHandler019HandleListTools');
  const ret = toolHandler019HandleListTools(variant2_0, variant2_1, variant2_2, variant4_0, variant4_1, variant4_2, variant9_0, variant9_1, variant9_2);
  endCurrentTask(0);
  let variant44;
  switch (dataView(memory0).getUint8(ret + 0, true)) {
    case 0: {
      var len33 = dataView(memory0).getUint32(ret + 8, true);
      var base33 = dataView(memory0).getUint32(ret + 4, true);
      var result33 = [];
      for (let i = 0; i < len33; i++) {
        const base = base33 + i * 88;
        var ptr10 = dataView(memory0).getUint32(base + 0, true);
        var len10 = dataView(memory0).getUint32(base + 4, true);
        var result10 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr10, len10));
        let variant12;
        switch (dataView(memory0).getUint8(base + 8, true)) {
          case 0: {
            variant12 = undefined;
            break;
          }
          case 1: {
            var ptr11 = dataView(memory0).getUint32(base + 12, true);
            var len11 = dataView(memory0).getUint32(base + 16, true);
            var result11 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr11, len11));
            variant12 = result11;
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for option');
          }
        }
        let variant14;
        switch (dataView(memory0).getUint8(base + 20, true)) {
          case 0: {
            variant14 = undefined;
            break;
          }
          case 1: {
            var ptr13 = dataView(memory0).getUint32(base + 24, true);
            var len13 = dataView(memory0).getUint32(base + 28, true);
            var result13 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr13, len13));
            variant14 = result13;
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for option');
          }
        }
        var ptr15 = dataView(memory0).getUint32(base + 32, true);
        var len15 = dataView(memory0).getUint32(base + 36, true);
        var result15 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr15, len15));
        let variant17;
        switch (dataView(memory0).getUint8(base + 40, true)) {
          case 0: {
            variant17 = undefined;
            break;
          }
          case 1: {
            var ptr16 = dataView(memory0).getUint32(base + 44, true);
            var len16 = dataView(memory0).getUint32(base + 48, true);
            var result16 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr16, len16));
            variant17 = result16;
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for option');
          }
        }
        let variant28;
        switch (dataView(memory0).getUint8(base + 52, true)) {
          case 0: {
            variant28 = undefined;
            break;
          }
          case 1: {
            let variant19;
            switch (dataView(memory0).getUint8(base + 56, true)) {
              case 0: {
                variant19 = undefined;
                break;
              }
              case 1: {
                var ptr18 = dataView(memory0).getUint32(base + 60, true);
                var len18 = dataView(memory0).getUint32(base + 64, true);
                var result18 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr18, len18));
                variant19 = result18;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant21;
            switch (dataView(memory0).getUint8(base + 68, true)) {
              case 0: {
                variant21 = undefined;
                break;
              }
              case 1: {
                var bool20 = dataView(memory0).getUint8(base + 69, true);
                variant21 = bool20 == 0 ? false : (bool20 == 1 ? true : throwInvalidBool());
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant23;
            switch (dataView(memory0).getUint8(base + 70, true)) {
              case 0: {
                variant23 = undefined;
                break;
              }
              case 1: {
                var bool22 = dataView(memory0).getUint8(base + 71, true);
                variant23 = bool22 == 0 ? false : (bool22 == 1 ? true : throwInvalidBool());
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant25;
            switch (dataView(memory0).getUint8(base + 72, true)) {
              case 0: {
                variant25 = undefined;
                break;
              }
              case 1: {
                var bool24 = dataView(memory0).getUint8(base + 73, true);
                variant25 = bool24 == 0 ? false : (bool24 == 1 ? true : throwInvalidBool());
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant27;
            switch (dataView(memory0).getUint8(base + 74, true)) {
              case 0: {
                variant27 = undefined;
                break;
              }
              case 1: {
                var bool26 = dataView(memory0).getUint8(base + 75, true);
                variant27 = bool26 == 0 ? false : (bool26 == 1 ? true : throwInvalidBool());
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant28 = {
              title: variant19,
              readOnlyHint: variant21,
              destructiveHint: variant23,
              idempotentHint: variant25,
              openWorldHint: variant27,
            };
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for option');
          }
        }
        let variant32;
        switch (dataView(memory0).getUint8(base + 76, true)) {
          case 0: {
            variant32 = undefined;
            break;
          }
          case 1: {
            var len31 = dataView(memory0).getUint32(base + 84, true);
            var base31 = dataView(memory0).getUint32(base + 80, true);
            var result31 = [];
            for (let i = 0; i < len31; i++) {
              const base = base31 + i * 16;
              var ptr29 = dataView(memory0).getUint32(base + 0, true);
              var len29 = dataView(memory0).getUint32(base + 4, true);
              var result29 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr29, len29));
              var ptr30 = dataView(memory0).getUint32(base + 8, true);
              var len30 = dataView(memory0).getUint32(base + 12, true);
              var result30 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr30, len30));
              result31.push([result29, result30]);
            }
            variant32 = result31;
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for option');
          }
        }
        result33.push({
          base: {
            name: result10,
            title: variant12,
          },
          description: variant14,
          inputSchema: result15,
          outputSchema: variant17,
          annotations: variant28,
          meta: variant32,
        });
      }
      let variant35;
      switch (dataView(memory0).getUint8(ret + 12, true)) {
        case 0: {
          variant35 = undefined;
          break;
        }
        case 1: {
          var ptr34 = dataView(memory0).getUint32(ret + 16, true);
          var len34 = dataView(memory0).getUint32(ret + 20, true);
          var result34 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr34, len34));
          variant35 = result34;
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      let variant39;
      switch (dataView(memory0).getUint8(ret + 24, true)) {
        case 0: {
          variant39 = undefined;
          break;
        }
        case 1: {
          var len38 = dataView(memory0).getUint32(ret + 32, true);
          var base38 = dataView(memory0).getUint32(ret + 28, true);
          var result38 = [];
          for (let i = 0; i < len38; i++) {
            const base = base38 + i * 16;
            var ptr36 = dataView(memory0).getUint32(base + 0, true);
            var len36 = dataView(memory0).getUint32(base + 4, true);
            var result36 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr36, len36));
            var ptr37 = dataView(memory0).getUint32(base + 8, true);
            var len37 = dataView(memory0).getUint32(base + 12, true);
            var result37 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr37, len37));
            result38.push([result36, result37]);
          }
          variant39 = result38;
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      variant44= {
        tag: 'ok',
        val: {
          tools: result33,
          nextCursor: variant35,
          meta: variant39,
        }
      };
      break;
    }
    case 1: {
      let variant40;
      switch (dataView(memory0).getUint8(ret + 4, true)) {
        case 0: {
          variant40= {
            tag: 'parse-error',
          };
          break;
        }
        case 1: {
          variant40= {
            tag: 'invalid-request',
          };
          break;
        }
        case 2: {
          variant40= {
            tag: 'method-not-found',
          };
          break;
        }
        case 3: {
          variant40= {
            tag: 'invalid-params',
          };
          break;
        }
        case 4: {
          variant40= {
            tag: 'internal-error',
          };
          break;
        }
        case 5: {
          variant40= {
            tag: 'resource-not-found',
          };
          break;
        }
        case 6: {
          variant40= {
            tag: 'tool-not-found',
          };
          break;
        }
        case 7: {
          variant40= {
            tag: 'prompt-not-found',
          };
          break;
        }
        case 8: {
          variant40= {
            tag: 'unauthorized',
          };
          break;
        }
        case 9: {
          variant40= {
            tag: 'rate-limited',
          };
          break;
        }
        case 10: {
          variant40= {
            tag: 'timeout',
          };
          break;
        }
        case 11: {
          variant40= {
            tag: 'cancelled',
          };
          break;
        }
        case 12: {
          variant40= {
            tag: 'custom-code',
            val: dataView(memory0).getInt32(ret + 8, true)
          };
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for ErrorCode');
        }
      }
      var ptr41 = dataView(memory0).getUint32(ret + 12, true);
      var len41 = dataView(memory0).getUint32(ret + 16, true);
      var result41 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr41, len41));
      let variant43;
      switch (dataView(memory0).getUint8(ret + 20, true)) {
        case 0: {
          variant43 = undefined;
          break;
        }
        case 1: {
          var ptr42 = dataView(memory0).getUint32(ret + 24, true);
          var len42 = dataView(memory0).getUint32(ret + 28, true);
          var result42 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr42, len42));
          variant43 = result42;
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      variant44= {
        tag: 'err',
        val: {
          code: variant40,
          message: result41,
          data: variant43,
        }
      };
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for expected');
    }
  }
  _debugLog('[iface="fastertools:mcp/tool-handler@0.1.9", function="handle-list-tools"][Instruction::Return]', {
    funcName: 'handle-list-tools',
    paramCount: 1,
    postReturn: true
  });
  const retCopy = variant44;
  
  let cstate = getOrCreateAsyncState(0);
  cstate.mayLeave = false;
  postReturn0(ret);
  cstate.mayLeave = true;
  
  
  
  if (typeof retCopy === 'object' && retCopy.tag === 'err') {
    throw new ComponentError(retCopy.val);
  }
  return retCopy.val;
  
}
let toolHandler019HandleCallTool;

function handleCallTool(arg0) {
  var {name: v0_0, arguments: v0_1, progressToken: v0_2, meta: v0_3 } = arg0;
  var ptr1 = utf8Encode(v0_0, realloc0, memory0);
  var len1 = utf8EncodedLen;
  var variant3 = v0_1;
  let variant3_0;
  let variant3_1;
  let variant3_2;
  if (variant3 === null || variant3=== undefined) {
    variant3_0 = 0;
    variant3_1 = 0;
    variant3_2 = 0;
  } else {
    const e = variant3;
    var ptr2 = utf8Encode(e, realloc0, memory0);
    var len2 = utf8EncodedLen;
    variant3_0 = 1;
    variant3_1 = ptr2;
    variant3_2 = len2;
  }
  var variant5 = v0_2;
  let variant5_0;
  let variant5_1;
  let variant5_2;
  if (variant5 === null || variant5=== undefined) {
    variant5_0 = 0;
    variant5_1 = 0;
    variant5_2 = 0;
  } else {
    const e = variant5;
    var ptr4 = utf8Encode(e, realloc0, memory0);
    var len4 = utf8EncodedLen;
    variant5_0 = 1;
    variant5_1 = ptr4;
    variant5_2 = len4;
  }
  var variant10 = v0_3;
  let variant10_0;
  let variant10_1;
  let variant10_2;
  if (variant10 === null || variant10=== undefined) {
    variant10_0 = 0;
    variant10_1 = 0;
    variant10_2 = 0;
  } else {
    const e = variant10;
    var vec9 = e;
    var len9 = vec9.length;
    var result9 = realloc0(0, 0, 4, len9 * 16);
    for (let i = 0; i < vec9.length; i++) {
      const e = vec9[i];
      const base = result9 + i * 16;var [tuple6_0, tuple6_1] = e;
      var ptr7 = utf8Encode(tuple6_0, realloc0, memory0);
      var len7 = utf8EncodedLen;
      dataView(memory0).setUint32(base + 4, len7, true);
      dataView(memory0).setUint32(base + 0, ptr7, true);
      var ptr8 = utf8Encode(tuple6_1, realloc0, memory0);
      var len8 = utf8EncodedLen;
      dataView(memory0).setUint32(base + 12, len8, true);
      dataView(memory0).setUint32(base + 8, ptr8, true);
    }
    variant10_0 = 1;
    variant10_1 = result9;
    variant10_2 = len9;
  }
  _debugLog('[iface="fastertools:mcp/tool-handler@0.1.9", function="handle-call-tool"] [Instruction::CallWasm] (async? false, @ enter)');
  const _wasm_call_currentTaskID = startCurrentTask(0, false, 'toolHandler019HandleCallTool');
  const ret = toolHandler019HandleCallTool(ptr1, len1, variant3_0, variant3_1, variant3_2, variant5_0, variant5_1, variant5_2, variant10_0, variant10_1, variant10_2);
  endCurrentTask(0);
  let variant111;
  switch (dataView(memory0).getUint8(ret + 0, true)) {
    case 0: {
      var len98 = dataView(memory0).getUint32(ret + 8, true);
      var base98 = dataView(memory0).getUint32(ret + 4, true);
      var result98 = [];
      for (let i = 0; i < len98; i++) {
        const base = base98 + i * 152;
        let variant97;
        switch (dataView(memory0).getUint8(base + 0, true)) {
          case 0: {
            var ptr11 = dataView(memory0).getUint32(base + 8, true);
            var len11 = dataView(memory0).getUint32(base + 12, true);
            var result11 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr11, len11));
            let variant18;
            switch (dataView(memory0).getUint8(base + 16, true)) {
              case 0: {
                variant18 = undefined;
                break;
              }
              case 1: {
                let variant14;
                switch (dataView(memory0).getUint8(base + 24, true)) {
                  case 0: {
                    variant14 = undefined;
                    break;
                  }
                  case 1: {
                    var len13 = dataView(memory0).getUint32(base + 32, true);
                    var base13 = dataView(memory0).getUint32(base + 28, true);
                    var result13 = [];
                    for (let i = 0; i < len13; i++) {
                      const base = base13 + i * 1;
                      let enum12;
                      switch (dataView(memory0).getUint8(base + 0, true)) {
                        case 0: {
                          enum12 = 'user';
                          break;
                        }
                        case 1: {
                          enum12 = 'assistant';
                          break;
                        }
                        default: {
                          throw new TypeError('invalid discriminant specified for Role');
                        }
                      }
                      result13.push(enum12);
                    }
                    variant14 = result13;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant15;
                switch (dataView(memory0).getUint8(base + 40, true)) {
                  case 0: {
                    variant15 = undefined;
                    break;
                  }
                  case 1: {
                    variant15 = dataView(memory0).getFloat64(base + 48, true);
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant17;
                switch (dataView(memory0).getUint8(base + 56, true)) {
                  case 0: {
                    variant17 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr16 = dataView(memory0).getUint32(base + 60, true);
                    var len16 = dataView(memory0).getUint32(base + 64, true);
                    var result16 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr16, len16));
                    variant17 = result16;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant18 = {
                  audience: variant14,
                  priority: variant15,
                  lastModified: variant17,
                };
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant22;
            switch (dataView(memory0).getUint8(base + 72, true)) {
              case 0: {
                variant22 = undefined;
                break;
              }
              case 1: {
                var len21 = dataView(memory0).getUint32(base + 80, true);
                var base21 = dataView(memory0).getUint32(base + 76, true);
                var result21 = [];
                for (let i = 0; i < len21; i++) {
                  const base = base21 + i * 16;
                  var ptr19 = dataView(memory0).getUint32(base + 0, true);
                  var len19 = dataView(memory0).getUint32(base + 4, true);
                  var result19 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr19, len19));
                  var ptr20 = dataView(memory0).getUint32(base + 8, true);
                  var len20 = dataView(memory0).getUint32(base + 12, true);
                  var result20 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr20, len20));
                  result21.push([result19, result20]);
                }
                variant22 = result21;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant97= {
              tag: 'text',
              val: {
                text: result11,
                annotations: variant18,
                meta: variant22,
              }
            };
            break;
          }
          case 1: {
            var ptr23 = dataView(memory0).getUint32(base + 8, true);
            var len23 = dataView(memory0).getUint32(base + 12, true);
            var result23 = new Uint8Array(memory0.buffer.slice(ptr23, ptr23 + len23 * 1));
            var ptr24 = dataView(memory0).getUint32(base + 16, true);
            var len24 = dataView(memory0).getUint32(base + 20, true);
            var result24 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr24, len24));
            let variant31;
            switch (dataView(memory0).getUint8(base + 24, true)) {
              case 0: {
                variant31 = undefined;
                break;
              }
              case 1: {
                let variant27;
                switch (dataView(memory0).getUint8(base + 32, true)) {
                  case 0: {
                    variant27 = undefined;
                    break;
                  }
                  case 1: {
                    var len26 = dataView(memory0).getUint32(base + 40, true);
                    var base26 = dataView(memory0).getUint32(base + 36, true);
                    var result26 = [];
                    for (let i = 0; i < len26; i++) {
                      const base = base26 + i * 1;
                      let enum25;
                      switch (dataView(memory0).getUint8(base + 0, true)) {
                        case 0: {
                          enum25 = 'user';
                          break;
                        }
                        case 1: {
                          enum25 = 'assistant';
                          break;
                        }
                        default: {
                          throw new TypeError('invalid discriminant specified for Role');
                        }
                      }
                      result26.push(enum25);
                    }
                    variant27 = result26;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant28;
                switch (dataView(memory0).getUint8(base + 48, true)) {
                  case 0: {
                    variant28 = undefined;
                    break;
                  }
                  case 1: {
                    variant28 = dataView(memory0).getFloat64(base + 56, true);
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant30;
                switch (dataView(memory0).getUint8(base + 64, true)) {
                  case 0: {
                    variant30 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr29 = dataView(memory0).getUint32(base + 68, true);
                    var len29 = dataView(memory0).getUint32(base + 72, true);
                    var result29 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr29, len29));
                    variant30 = result29;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant31 = {
                  audience: variant27,
                  priority: variant28,
                  lastModified: variant30,
                };
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant35;
            switch (dataView(memory0).getUint8(base + 80, true)) {
              case 0: {
                variant35 = undefined;
                break;
              }
              case 1: {
                var len34 = dataView(memory0).getUint32(base + 88, true);
                var base34 = dataView(memory0).getUint32(base + 84, true);
                var result34 = [];
                for (let i = 0; i < len34; i++) {
                  const base = base34 + i * 16;
                  var ptr32 = dataView(memory0).getUint32(base + 0, true);
                  var len32 = dataView(memory0).getUint32(base + 4, true);
                  var result32 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr32, len32));
                  var ptr33 = dataView(memory0).getUint32(base + 8, true);
                  var len33 = dataView(memory0).getUint32(base + 12, true);
                  var result33 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr33, len33));
                  result34.push([result32, result33]);
                }
                variant35 = result34;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant97= {
              tag: 'image',
              val: {
                data: result23,
                mimeType: result24,
                annotations: variant31,
                meta: variant35,
              }
            };
            break;
          }
          case 2: {
            var ptr36 = dataView(memory0).getUint32(base + 8, true);
            var len36 = dataView(memory0).getUint32(base + 12, true);
            var result36 = new Uint8Array(memory0.buffer.slice(ptr36, ptr36 + len36 * 1));
            var ptr37 = dataView(memory0).getUint32(base + 16, true);
            var len37 = dataView(memory0).getUint32(base + 20, true);
            var result37 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr37, len37));
            let variant44;
            switch (dataView(memory0).getUint8(base + 24, true)) {
              case 0: {
                variant44 = undefined;
                break;
              }
              case 1: {
                let variant40;
                switch (dataView(memory0).getUint8(base + 32, true)) {
                  case 0: {
                    variant40 = undefined;
                    break;
                  }
                  case 1: {
                    var len39 = dataView(memory0).getUint32(base + 40, true);
                    var base39 = dataView(memory0).getUint32(base + 36, true);
                    var result39 = [];
                    for (let i = 0; i < len39; i++) {
                      const base = base39 + i * 1;
                      let enum38;
                      switch (dataView(memory0).getUint8(base + 0, true)) {
                        case 0: {
                          enum38 = 'user';
                          break;
                        }
                        case 1: {
                          enum38 = 'assistant';
                          break;
                        }
                        default: {
                          throw new TypeError('invalid discriminant specified for Role');
                        }
                      }
                      result39.push(enum38);
                    }
                    variant40 = result39;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant41;
                switch (dataView(memory0).getUint8(base + 48, true)) {
                  case 0: {
                    variant41 = undefined;
                    break;
                  }
                  case 1: {
                    variant41 = dataView(memory0).getFloat64(base + 56, true);
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant43;
                switch (dataView(memory0).getUint8(base + 64, true)) {
                  case 0: {
                    variant43 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr42 = dataView(memory0).getUint32(base + 68, true);
                    var len42 = dataView(memory0).getUint32(base + 72, true);
                    var result42 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr42, len42));
                    variant43 = result42;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant44 = {
                  audience: variant40,
                  priority: variant41,
                  lastModified: variant43,
                };
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant48;
            switch (dataView(memory0).getUint8(base + 80, true)) {
              case 0: {
                variant48 = undefined;
                break;
              }
              case 1: {
                var len47 = dataView(memory0).getUint32(base + 88, true);
                var base47 = dataView(memory0).getUint32(base + 84, true);
                var result47 = [];
                for (let i = 0; i < len47; i++) {
                  const base = base47 + i * 16;
                  var ptr45 = dataView(memory0).getUint32(base + 0, true);
                  var len45 = dataView(memory0).getUint32(base + 4, true);
                  var result45 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr45, len45));
                  var ptr46 = dataView(memory0).getUint32(base + 8, true);
                  var len46 = dataView(memory0).getUint32(base + 12, true);
                  var result46 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr46, len46));
                  result47.push([result45, result46]);
                }
                variant48 = result47;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant97= {
              tag: 'audio',
              val: {
                data: result36,
                mimeType: result37,
                annotations: variant44,
                meta: variant48,
              }
            };
            break;
          }
          case 3: {
            var ptr49 = dataView(memory0).getUint32(base + 8, true);
            var len49 = dataView(memory0).getUint32(base + 12, true);
            var result49 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr49, len49));
            var ptr50 = dataView(memory0).getUint32(base + 16, true);
            var len50 = dataView(memory0).getUint32(base + 20, true);
            var result50 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr50, len50));
            let variant52;
            switch (dataView(memory0).getUint8(base + 24, true)) {
              case 0: {
                variant52 = undefined;
                break;
              }
              case 1: {
                var ptr51 = dataView(memory0).getUint32(base + 28, true);
                var len51 = dataView(memory0).getUint32(base + 32, true);
                var result51 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr51, len51));
                variant52 = result51;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant54;
            switch (dataView(memory0).getUint8(base + 36, true)) {
              case 0: {
                variant54 = undefined;
                break;
              }
              case 1: {
                var ptr53 = dataView(memory0).getUint32(base + 40, true);
                var len53 = dataView(memory0).getUint32(base + 44, true);
                var result53 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr53, len53));
                variant54 = result53;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant56;
            switch (dataView(memory0).getUint8(base + 48, true)) {
              case 0: {
                variant56 = undefined;
                break;
              }
              case 1: {
                var ptr55 = dataView(memory0).getUint32(base + 52, true);
                var len55 = dataView(memory0).getUint32(base + 56, true);
                var result55 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr55, len55));
                variant56 = result55;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant57;
            switch (dataView(memory0).getUint8(base + 64, true)) {
              case 0: {
                variant57 = undefined;
                break;
              }
              case 1: {
                variant57 = BigInt.asUintN(64, dataView(memory0).getBigInt64(base + 72, true));
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant64;
            switch (dataView(memory0).getUint8(base + 80, true)) {
              case 0: {
                variant64 = undefined;
                break;
              }
              case 1: {
                let variant60;
                switch (dataView(memory0).getUint8(base + 88, true)) {
                  case 0: {
                    variant60 = undefined;
                    break;
                  }
                  case 1: {
                    var len59 = dataView(memory0).getUint32(base + 96, true);
                    var base59 = dataView(memory0).getUint32(base + 92, true);
                    var result59 = [];
                    for (let i = 0; i < len59; i++) {
                      const base = base59 + i * 1;
                      let enum58;
                      switch (dataView(memory0).getUint8(base + 0, true)) {
                        case 0: {
                          enum58 = 'user';
                          break;
                        }
                        case 1: {
                          enum58 = 'assistant';
                          break;
                        }
                        default: {
                          throw new TypeError('invalid discriminant specified for Role');
                        }
                      }
                      result59.push(enum58);
                    }
                    variant60 = result59;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant61;
                switch (dataView(memory0).getUint8(base + 104, true)) {
                  case 0: {
                    variant61 = undefined;
                    break;
                  }
                  case 1: {
                    variant61 = dataView(memory0).getFloat64(base + 112, true);
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant63;
                switch (dataView(memory0).getUint8(base + 120, true)) {
                  case 0: {
                    variant63 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr62 = dataView(memory0).getUint32(base + 124, true);
                    var len62 = dataView(memory0).getUint32(base + 128, true);
                    var result62 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr62, len62));
                    variant63 = result62;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant64 = {
                  audience: variant60,
                  priority: variant61,
                  lastModified: variant63,
                };
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant68;
            switch (dataView(memory0).getUint8(base + 136, true)) {
              case 0: {
                variant68 = undefined;
                break;
              }
              case 1: {
                var len67 = dataView(memory0).getUint32(base + 144, true);
                var base67 = dataView(memory0).getUint32(base + 140, true);
                var result67 = [];
                for (let i = 0; i < len67; i++) {
                  const base = base67 + i * 16;
                  var ptr65 = dataView(memory0).getUint32(base + 0, true);
                  var len65 = dataView(memory0).getUint32(base + 4, true);
                  var result65 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr65, len65));
                  var ptr66 = dataView(memory0).getUint32(base + 8, true);
                  var len66 = dataView(memory0).getUint32(base + 12, true);
                  var result66 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr66, len66));
                  result67.push([result65, result66]);
                }
                variant68 = result67;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant97= {
              tag: 'resource-link',
              val: {
                uri: result49,
                name: result50,
                title: variant52,
                description: variant54,
                mimeType: variant56,
                size: variant57,
                annotations: variant64,
                meta: variant68,
              }
            };
            break;
          }
          case 4: {
            let variant85;
            switch (dataView(memory0).getUint8(base + 8, true)) {
              case 0: {
                var ptr69 = dataView(memory0).getUint32(base + 12, true);
                var len69 = dataView(memory0).getUint32(base + 16, true);
                var result69 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr69, len69));
                let variant71;
                switch (dataView(memory0).getUint8(base + 20, true)) {
                  case 0: {
                    variant71 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr70 = dataView(memory0).getUint32(base + 24, true);
                    var len70 = dataView(memory0).getUint32(base + 28, true);
                    var result70 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr70, len70));
                    variant71 = result70;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                var ptr72 = dataView(memory0).getUint32(base + 32, true);
                var len72 = dataView(memory0).getUint32(base + 36, true);
                var result72 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr72, len72));
                let variant76;
                switch (dataView(memory0).getUint8(base + 40, true)) {
                  case 0: {
                    variant76 = undefined;
                    break;
                  }
                  case 1: {
                    var len75 = dataView(memory0).getUint32(base + 48, true);
                    var base75 = dataView(memory0).getUint32(base + 44, true);
                    var result75 = [];
                    for (let i = 0; i < len75; i++) {
                      const base = base75 + i * 16;
                      var ptr73 = dataView(memory0).getUint32(base + 0, true);
                      var len73 = dataView(memory0).getUint32(base + 4, true);
                      var result73 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr73, len73));
                      var ptr74 = dataView(memory0).getUint32(base + 8, true);
                      var len74 = dataView(memory0).getUint32(base + 12, true);
                      var result74 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr74, len74));
                      result75.push([result73, result74]);
                    }
                    variant76 = result75;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant85= {
                  tag: 'text',
                  val: {
                    uri: result69,
                    mimeType: variant71,
                    text: result72,
                    meta: variant76,
                  }
                };
                break;
              }
              case 1: {
                var ptr77 = dataView(memory0).getUint32(base + 12, true);
                var len77 = dataView(memory0).getUint32(base + 16, true);
                var result77 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr77, len77));
                let variant79;
                switch (dataView(memory0).getUint8(base + 20, true)) {
                  case 0: {
                    variant79 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr78 = dataView(memory0).getUint32(base + 24, true);
                    var len78 = dataView(memory0).getUint32(base + 28, true);
                    var result78 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr78, len78));
                    variant79 = result78;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                var ptr80 = dataView(memory0).getUint32(base + 32, true);
                var len80 = dataView(memory0).getUint32(base + 36, true);
                var result80 = new Uint8Array(memory0.buffer.slice(ptr80, ptr80 + len80 * 1));
                let variant84;
                switch (dataView(memory0).getUint8(base + 40, true)) {
                  case 0: {
                    variant84 = undefined;
                    break;
                  }
                  case 1: {
                    var len83 = dataView(memory0).getUint32(base + 48, true);
                    var base83 = dataView(memory0).getUint32(base + 44, true);
                    var result83 = [];
                    for (let i = 0; i < len83; i++) {
                      const base = base83 + i * 16;
                      var ptr81 = dataView(memory0).getUint32(base + 0, true);
                      var len81 = dataView(memory0).getUint32(base + 4, true);
                      var result81 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr81, len81));
                      var ptr82 = dataView(memory0).getUint32(base + 8, true);
                      var len82 = dataView(memory0).getUint32(base + 12, true);
                      var result82 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr82, len82));
                      result83.push([result81, result82]);
                    }
                    variant84 = result83;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant85= {
                  tag: 'blob',
                  val: {
                    uri: result77,
                    mimeType: variant79,
                    blob: result80,
                    meta: variant84,
                  }
                };
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for ResourceContents');
              }
            }
            let variant92;
            switch (dataView(memory0).getUint8(base + 56, true)) {
              case 0: {
                variant92 = undefined;
                break;
              }
              case 1: {
                let variant88;
                switch (dataView(memory0).getUint8(base + 64, true)) {
                  case 0: {
                    variant88 = undefined;
                    break;
                  }
                  case 1: {
                    var len87 = dataView(memory0).getUint32(base + 72, true);
                    var base87 = dataView(memory0).getUint32(base + 68, true);
                    var result87 = [];
                    for (let i = 0; i < len87; i++) {
                      const base = base87 + i * 1;
                      let enum86;
                      switch (dataView(memory0).getUint8(base + 0, true)) {
                        case 0: {
                          enum86 = 'user';
                          break;
                        }
                        case 1: {
                          enum86 = 'assistant';
                          break;
                        }
                        default: {
                          throw new TypeError('invalid discriminant specified for Role');
                        }
                      }
                      result87.push(enum86);
                    }
                    variant88 = result87;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant89;
                switch (dataView(memory0).getUint8(base + 80, true)) {
                  case 0: {
                    variant89 = undefined;
                    break;
                  }
                  case 1: {
                    variant89 = dataView(memory0).getFloat64(base + 88, true);
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                let variant91;
                switch (dataView(memory0).getUint8(base + 96, true)) {
                  case 0: {
                    variant91 = undefined;
                    break;
                  }
                  case 1: {
                    var ptr90 = dataView(memory0).getUint32(base + 100, true);
                    var len90 = dataView(memory0).getUint32(base + 104, true);
                    var result90 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr90, len90));
                    variant91 = result90;
                    break;
                  }
                  default: {
                    throw new TypeError('invalid variant discriminant for option');
                  }
                }
                variant92 = {
                  audience: variant88,
                  priority: variant89,
                  lastModified: variant91,
                };
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            let variant96;
            switch (dataView(memory0).getUint8(base + 112, true)) {
              case 0: {
                variant96 = undefined;
                break;
              }
              case 1: {
                var len95 = dataView(memory0).getUint32(base + 120, true);
                var base95 = dataView(memory0).getUint32(base + 116, true);
                var result95 = [];
                for (let i = 0; i < len95; i++) {
                  const base = base95 + i * 16;
                  var ptr93 = dataView(memory0).getUint32(base + 0, true);
                  var len93 = dataView(memory0).getUint32(base + 4, true);
                  var result93 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr93, len93));
                  var ptr94 = dataView(memory0).getUint32(base + 8, true);
                  var len94 = dataView(memory0).getUint32(base + 12, true);
                  var result94 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr94, len94));
                  result95.push([result93, result94]);
                }
                variant96 = result95;
                break;
              }
              default: {
                throw new TypeError('invalid variant discriminant for option');
              }
            }
            variant97= {
              tag: 'embedded-resource',
              val: {
                contents: variant85,
                annotations: variant92,
                meta: variant96,
              }
            };
            break;
          }
          default: {
            throw new TypeError('invalid variant discriminant for ContentBlock');
          }
        }
        result98.push(variant97);
      }
      let variant100;
      switch (dataView(memory0).getUint8(ret + 12, true)) {
        case 0: {
          variant100 = undefined;
          break;
        }
        case 1: {
          var ptr99 = dataView(memory0).getUint32(ret + 16, true);
          var len99 = dataView(memory0).getUint32(ret + 20, true);
          var result99 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr99, len99));
          variant100 = result99;
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      let variant102;
      switch (dataView(memory0).getUint8(ret + 24, true)) {
        case 0: {
          variant102 = undefined;
          break;
        }
        case 1: {
          var bool101 = dataView(memory0).getUint8(ret + 25, true);
          variant102 = bool101 == 0 ? false : (bool101 == 1 ? true : throwInvalidBool());
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      let variant106;
      switch (dataView(memory0).getUint8(ret + 28, true)) {
        case 0: {
          variant106 = undefined;
          break;
        }
        case 1: {
          var len105 = dataView(memory0).getUint32(ret + 36, true);
          var base105 = dataView(memory0).getUint32(ret + 32, true);
          var result105 = [];
          for (let i = 0; i < len105; i++) {
            const base = base105 + i * 16;
            var ptr103 = dataView(memory0).getUint32(base + 0, true);
            var len103 = dataView(memory0).getUint32(base + 4, true);
            var result103 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr103, len103));
            var ptr104 = dataView(memory0).getUint32(base + 8, true);
            var len104 = dataView(memory0).getUint32(base + 12, true);
            var result104 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr104, len104));
            result105.push([result103, result104]);
          }
          variant106 = result105;
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      variant111= {
        tag: 'ok',
        val: {
          content: result98,
          structuredContent: variant100,
          isError: variant102,
          meta: variant106,
        }
      };
      break;
    }
    case 1: {
      let variant107;
      switch (dataView(memory0).getUint8(ret + 4, true)) {
        case 0: {
          variant107= {
            tag: 'parse-error',
          };
          break;
        }
        case 1: {
          variant107= {
            tag: 'invalid-request',
          };
          break;
        }
        case 2: {
          variant107= {
            tag: 'method-not-found',
          };
          break;
        }
        case 3: {
          variant107= {
            tag: 'invalid-params',
          };
          break;
        }
        case 4: {
          variant107= {
            tag: 'internal-error',
          };
          break;
        }
        case 5: {
          variant107= {
            tag: 'resource-not-found',
          };
          break;
        }
        case 6: {
          variant107= {
            tag: 'tool-not-found',
          };
          break;
        }
        case 7: {
          variant107= {
            tag: 'prompt-not-found',
          };
          break;
        }
        case 8: {
          variant107= {
            tag: 'unauthorized',
          };
          break;
        }
        case 9: {
          variant107= {
            tag: 'rate-limited',
          };
          break;
        }
        case 10: {
          variant107= {
            tag: 'timeout',
          };
          break;
        }
        case 11: {
          variant107= {
            tag: 'cancelled',
          };
          break;
        }
        case 12: {
          variant107= {
            tag: 'custom-code',
            val: dataView(memory0).getInt32(ret + 8, true)
          };
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for ErrorCode');
        }
      }
      var ptr108 = dataView(memory0).getUint32(ret + 12, true);
      var len108 = dataView(memory0).getUint32(ret + 16, true);
      var result108 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr108, len108));
      let variant110;
      switch (dataView(memory0).getUint8(ret + 20, true)) {
        case 0: {
          variant110 = undefined;
          break;
        }
        case 1: {
          var ptr109 = dataView(memory0).getUint32(ret + 24, true);
          var len109 = dataView(memory0).getUint32(ret + 28, true);
          var result109 = utf8Decoder.decode(new Uint8Array(memory0.buffer, ptr109, len109));
          variant110 = result109;
          break;
        }
        default: {
          throw new TypeError('invalid variant discriminant for option');
        }
      }
      variant111= {
        tag: 'err',
        val: {
          code: variant107,
          message: result108,
          data: variant110,
        }
      };
      break;
    }
    default: {
      throw new TypeError('invalid variant discriminant for expected');
    }
  }
  _debugLog('[iface="fastertools:mcp/tool-handler@0.1.9", function="handle-call-tool"][Instruction::Return]', {
    funcName: 'handle-call-tool',
    paramCount: 1,
    postReturn: true
  });
  const retCopy = variant111;
  
  let cstate = getOrCreateAsyncState(0);
  cstate.mayLeave = false;
  postReturn1(ret);
  cstate.mayLeave = true;
  
  
  
  if (typeof retCopy === 'object' && retCopy.tag === 'err') {
    throw new ComponentError(retCopy.val);
  }
  return retCopy.val;
  
}

const $init = (() => {
  let gen = (function* init () {
    const module0 = fetchCompile(new URL('./weather-handler.core.wasm', import.meta.url));
    const module1 = fetchCompile(new URL('./weather-handler.core2.wasm', import.meta.url));
    const module2 = base64Compile('AGFzbQEAAAABaw9gA39/fwBgA39+fwBgAn9/AGAEf39/fwBgAn5/AGADf39/AX9gBn9/f39/fwBgBH9/f38Bf2AFf39/f38Bf2AJf39/f35/f39/AGAEf39/fwBgBH9/f38Bf2ACf38Bf2ADf35/AX9gAX8AAzU0AAEBAgMCBAADBQYDBgICAgICAgIHBwgHCQICAgIKAgoLDA0MDg4CAgIBAgICAwMCDg4ODgQFAXABNDQHhgI1ATAAAAExAAEBMgACATMAAwE0AAQBNQAFATYABgE3AAcBOAAIATkACQIxMAAKAjExAAsCMTIADAIxMwANAjE0AA4CMTUADwIxNgAQAjE3ABECMTgAEgIxOQATAjIwABQCMjEAFQIyMgAWAjIzABcCMjQAGAIyNQAZAjI2ABoCMjcAGwIyOAAcAjI5AB0CMzAAHgIzMQAfAjMyACACMzMAIQIzNAAiAjM1ACMCMzYAJAIzNwAlAjM4ACYCMzkAJwI0MAAoAjQxACkCNDIAKgI0MwArAjQ0ACwCNDUALQI0NgAuAjQ3AC8CNDgAMAI0OQAxAjUwADICNTEAMwgkaW1wb3J0cwEACsMFNA0AIAAgASACQQARAAALDQAgACABIAJBAREBAAsNACAAIAEgAkECEQEACwsAIAAgAUEDEQIACw8AIAAgASACIANBBBEDAAsLACAAIAFBBRECAAsLACAAIAFBBhEEAAsNACAAIAEgAkEHEQAACw8AIAAgASACIANBCBEDAAsNACAAIAEgAkEJEQUACxMAIAAgASACIAMgBCAFQQoRBgALDwAgACABIAIgA0ELEQMACxMAIAAgASACIAMgBCAFQQwRBgALCwAgACABQQ0RAgALCwAgACABQQ4RAgALCwAgACABQQ8RAgALCwAgACABQRARAgALCwAgACABQRERAgALCwAgACABQRIRAgALCwAgACABQRMRAgALDwAgACABIAIgA0EUEQcACw8AIAAgASACIANBFREHAAsRACAAIAEgAiADIARBFhEIAAsPACAAIAEgAiADQRcRBwALGQAgACABIAIgAyAEIAUgBiAHIAhBGBEJAAsLACAAIAFBGRECAAsLACAAIAFBGhECAAsLACAAIAFBGxECAAsLACAAIAFBHBECAAsPACAAIAEgAiADQR0RCgALCwAgACABQR4RAgALDwAgACABIAIgA0EfEQoACw8AIAAgASACIANBIBELAAsLACAAIAFBIREMAAsNACAAIAEgAkEiEQ0ACwsAIAAgAUEjEQwACwkAIABBJBEOAAsJACAAQSURDgALCwAgACABQSYRAgALCwAgACABQScRAgALCwAgACABQSgRAgALDQAgACABIAJBKREBAAsLACAAIAFBKhECAAsLACAAIAFBKxECAAsLACAAIAFBLBECAAsPACAAIAEgAiADQS0RAwALDwAgACABIAIgA0EuEQMACwsAIAAgAUEvEQIACwkAIABBMBEOAAsJACAAQTERDgALCQAgAEEyEQ4ACwkAIABBMxEOAAsALwlwcm9kdWNlcnMBDHByb2Nlc3NlZC1ieQENd2l0LWNvbXBvbmVudAcwLjIzNi4xANkYBG5hbWUAExJ3aXQtY29tcG9uZW50OnNoaW0BvBg0ACBpbmRpcmVjdC13YXNpOmlvL3BvbGxAMC4yLjMtcG9sbAE4aW5kaXJlY3Qtd2FzaTppby9zdHJlYW1zQDAuMi4zLVttZXRob2RdaW5wdXQtc3RyZWFtLnJlYWQCQWluZGlyZWN0LXdhc2k6aW8vc3RyZWFtc0AwLjIuMy1bbWV0aG9kXWlucHV0LXN0cmVhbS5ibG9ja2luZy1yZWFkA0BpbmRpcmVjdC13YXNpOmlvL3N0cmVhbXNAMC4yLjMtW21ldGhvZF1vdXRwdXQtc3RyZWFtLmNoZWNrLXdyaXRlBDppbmRpcmVjdC13YXNpOmlvL3N0cmVhbXNAMC4yLjMtW21ldGhvZF1vdXRwdXQtc3RyZWFtLndyaXRlBUNpbmRpcmVjdC13YXNpOmlvL3N0cmVhbXNAMC4yLjMtW21ldGhvZF1vdXRwdXQtc3RyZWFtLmJsb2NraW5nLWZsdXNoBjJpbmRpcmVjdC13YXNpOnJhbmRvbS9yYW5kb21AMC4yLjMtZ2V0LXJhbmRvbS1ieXRlcwc3aW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVtzdGF0aWNdZmllbGRzLmZyb20tbGlzdAgxaW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZmllbGRzLmdldAkxaW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZmllbGRzLmhhcwoxaW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZmllbGRzLnNldAs0aW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZmllbGRzLmRlbGV0ZQw0aW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZmllbGRzLmFwcGVuZA01aW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZmllbGRzLmVudHJpZXMOPmluZGlyZWN0LXdhc2k6aHR0cC90eXBlc0AwLjIuMy1bbWV0aG9kXWluY29taW5nLXJlcXVlc3QubWV0aG9kD0dpbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW21ldGhvZF1pbmNvbWluZy1yZXF1ZXN0LnBhdGgtd2l0aC1xdWVyeRA+aW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdaW5jb21pbmctcmVxdWVzdC5zY2hlbWURQWluZGlyZWN0LXdhc2k6aHR0cC90eXBlc0AwLjIuMy1bbWV0aG9kXWluY29taW5nLXJlcXVlc3QuYXV0aG9yaXR5Ej9pbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW21ldGhvZF1pbmNvbWluZy1yZXF1ZXN0LmNvbnN1bWUTPGluZGlyZWN0LXdhc2k6aHR0cC90eXBlc0AwLjIuMy1bbWV0aG9kXW91dGdvaW5nLXJlcXVlc3QuYm9keRRCaW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2Rdb3V0Z29pbmctcmVxdWVzdC5zZXQtbWV0aG9kFUtpbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW21ldGhvZF1vdXRnb2luZy1yZXF1ZXN0LnNldC1wYXRoLXdpdGgtcXVlcnkWQmluZGlyZWN0LXdhc2k6aHR0cC90eXBlc0AwLjIuMy1bbWV0aG9kXW91dGdvaW5nLXJlcXVlc3Quc2V0LXNjaGVtZRdFaW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2Rdb3V0Z29pbmctcmVxdWVzdC5zZXQtYXV0aG9yaXR5GDxpbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW3N0YXRpY11yZXNwb25zZS1vdXRwYXJhbS5zZXQZQGluZGlyZWN0LXdhc2k6aHR0cC90eXBlc0AwLjIuMy1bbWV0aG9kXWluY29taW5nLXJlc3BvbnNlLmNvbnN1bWUaO2luZGlyZWN0LXdhc2k6aHR0cC90eXBlc0AwLjIuMy1bbWV0aG9kXWluY29taW5nLWJvZHkuc3RyZWFtGz1pbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW21ldGhvZF1vdXRnb2luZy1yZXNwb25zZS5ib2R5HDppbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW21ldGhvZF1vdXRnb2luZy1ib2R5LndyaXRlHTtpbmRpcmVjdC13YXNpOmh0dHAvdHlwZXNAMC4yLjMtW3N0YXRpY11vdXRnb2luZy1ib2R5LmZpbmlzaB5DaW5kaXJlY3Qtd2FzaTpodHRwL3R5cGVzQDAuMi4zLVttZXRob2RdZnV0dXJlLWluY29taW5nLXJlc3BvbnNlLmdldB8waW5kaXJlY3Qtd2FzaTpodHRwL291dGdvaW5nLWhhbmRsZXJAMC4yLjMtaGFuZGxlICVhZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWZkX3dyaXRlISphZGFwdC13YXNpX3NuYXBzaG90X3ByZXZpZXcxLWNsb2NrX3Jlc19nZXQiK2FkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtY2xvY2tfdGltZV9nZXQjKmFkYXB0LXdhc2lfc25hcHNob3RfcHJldmlldzEtZmRfZmRzdGF0X2dldCQwaW5kaXJlY3Qtd2FzaTpjbG9ja3Mvd2FsbC1jbG9ja0AwLjIuMy1yZXNvbHV0aW9uJSlpbmRpcmVjdC13YXNpOmNsb2Nrcy93YWxsLWNsb2NrQDAuMi4zLW5vdyZBaW5kaXJlY3Qtd2FzaTpmaWxlc3lzdGVtL3R5cGVzQDAuMi4zLVttZXRob2RdZGVzY3JpcHRvci5nZXQtZmxhZ3MnQGluZGlyZWN0LXdhc2k6ZmlsZXN5c3RlbS90eXBlc0AwLjIuMy1bbWV0aG9kXWRlc2NyaXB0b3IuZ2V0LXR5cGUoOmluZGlyZWN0LXdhc2k6ZmlsZXN5c3RlbS90eXBlc0AwLjIuMy1maWxlc3lzdGVtLWVycm9yLWNvZGUpSGluZGlyZWN0LXdhc2k6ZmlsZXN5c3RlbS90eXBlc0AwLjIuMy1bbWV0aG9kXWRlc2NyaXB0b3Iud3JpdGUtdmlhLXN0cmVhbSpJaW5kaXJlY3Qtd2FzaTpmaWxlc3lzdGVtL3R5cGVzQDAuMi4zLVttZXRob2RdZGVzY3JpcHRvci5hcHBlbmQtdmlhLXN0cmVhbSs8aW5kaXJlY3Qtd2FzaTpmaWxlc3lzdGVtL3R5cGVzQDAuMi4zLVttZXRob2RdZGVzY3JpcHRvci5zdGF0LEBpbmRpcmVjdC13YXNpOmlvL3N0cmVhbXNAMC4yLjMtW21ldGhvZF1vdXRwdXQtc3RyZWFtLmNoZWNrLXdyaXRlLTppbmRpcmVjdC13YXNpOmlvL3N0cmVhbXNAMC4yLjMtW21ldGhvZF1vdXRwdXQtc3RyZWFtLndyaXRlLk1pbmRpcmVjdC13YXNpOmlvL3N0cmVhbXNAMC4yLjMtW21ldGhvZF1vdXRwdXQtc3RyZWFtLmJsb2NraW5nLXdyaXRlLWFuZC1mbHVzaC9DaW5kaXJlY3Qtd2FzaTppby9zdHJlYW1zQDAuMi4zLVttZXRob2Rdb3V0cHV0LXN0cmVhbS5ibG9ja2luZy1mbHVzaDA3aW5kaXJlY3Qtd2FzaTpmaWxlc3lzdGVtL3ByZW9wZW5zQDAuMi4zLWdldC1kaXJlY3RvcmllczE5aW5kaXJlY3Qtd2FzaTpjbGkvdGVybWluYWwtc3RkaW5AMC4yLjMtZ2V0LXRlcm1pbmFsLXN0ZGluMjtpbmRpcmVjdC13YXNpOmNsaS90ZXJtaW5hbC1zdGRvdXRAMC4yLjMtZ2V0LXRlcm1pbmFsLXN0ZG91dDM7aW5kaXJlY3Qtd2FzaTpjbGkvdGVybWluYWwtc3RkZXJyQDAuMi4zLWdldC10ZXJtaW5hbC1zdGRlcnI');
    const module3 = base64Compile('AGFzbQEAAAABaw9gA39/fwBgA39+fwBgAn9/AGAEf39/fwBgAn5/AGADf39/AX9gBn9/f39/fwBgBH9/f38Bf2AFf39/f38Bf2AJf39/f35/f39/AGAEf39/fwBgBH9/f38Bf2ACf38Bf2ADf35/AX9gAX8AAr4CNQABMAAAAAExAAEAATIAAQABMwACAAE0AAMAATUAAgABNgAEAAE3AAAAATgAAwABOQAFAAIxMAAGAAIxMQADAAIxMgAGAAIxMwACAAIxNAACAAIxNQACAAIxNgACAAIxNwACAAIxOAACAAIxOQACAAIyMAAHAAIyMQAHAAIyMgAIAAIyMwAHAAIyNAAJAAIyNQACAAIyNgACAAIyNwACAAIyOAACAAIyOQAKAAIzMAACAAIzMQAKAAIzMgALAAIzMwAMAAIzNAANAAIzNQAMAAIzNgAOAAIzNwAOAAIzOAACAAIzOQACAAI0MAACAAI0MQABAAI0MgACAAI0MwACAAI0NAACAAI0NQADAAI0NgADAAI0NwACAAI0OAAOAAI0OQAOAAI1MAAOAAI1MQAOAAgkaW1wb3J0cwFwATQ0CToBAEEACzQAAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyAhIiMkJSYnKCkqKywtLi8wMTIzAC8JcHJvZHVjZXJzAQxwcm9jZXNzZWQtYnkBDXdpdC1jb21wb25lbnQHMC4yMzYuMQAcBG5hbWUAFRR3aXQtY29tcG9uZW50OmZpeHVwcw');
    ({ exports: exports0 } = yield instantiateCore(yield module2));
    ({ exports: exports1 } = yield instantiateCore(yield module0, {
      'wasi:clocks/monotonic-clock@0.2.3': {
        now: trampoline6,
        'subscribe-duration': trampoline8,
        'subscribe-instant': trampoline7,
      },
      'wasi:http/outgoing-handler@0.2.3': {
        handle: exports0['31'],
      },
      'wasi:http/types@0.2.3': {
        '[constructor]fields': trampoline10,
        '[constructor]outgoing-request': trampoline13,
        '[constructor]outgoing-response': trampoline17,
        '[method]fields.append': exports0['12'],
        '[method]fields.clone': trampoline11,
        '[method]fields.delete': exports0['11'],
        '[method]fields.entries': exports0['13'],
        '[method]fields.get': exports0['8'],
        '[method]fields.has': exports0['9'],
        '[method]fields.set': exports0['10'],
        '[method]future-incoming-response.get': exports0['30'],
        '[method]future-incoming-response.subscribe': trampoline20,
        '[method]incoming-body.stream': exports0['26'],
        '[method]incoming-request.authority': exports0['17'],
        '[method]incoming-request.consume': exports0['18'],
        '[method]incoming-request.headers': trampoline12,
        '[method]incoming-request.method': exports0['14'],
        '[method]incoming-request.path-with-query': exports0['15'],
        '[method]incoming-request.scheme': exports0['16'],
        '[method]incoming-response.consume': exports0['25'],
        '[method]incoming-response.headers': trampoline16,
        '[method]incoming-response.status': trampoline15,
        '[method]outgoing-body.write': exports0['28'],
        '[method]outgoing-request.body': exports0['19'],
        '[method]outgoing-request.headers': trampoline14,
        '[method]outgoing-request.set-authority': exports0['23'],
        '[method]outgoing-request.set-method': exports0['20'],
        '[method]outgoing-request.set-path-with-query': exports0['21'],
        '[method]outgoing-request.set-scheme': exports0['22'],
        '[method]outgoing-response.body': exports0['27'],
        '[method]outgoing-response.headers': trampoline19,
        '[method]outgoing-response.set-status-code': trampoline18,
        '[static]fields.from-list': exports0['7'],
        '[static]outgoing-body.finish': exports0['29'],
        '[static]response-outparam.set': exports0['24'],
      },
      'wasi:io/poll@0.2.3': {
        '[method]pollable.block': trampoline3,
        '[resource-drop]pollable': trampoline0,
        poll: exports0['0'],
      },
      'wasi:io/streams@0.2.3': {
        '[method]input-stream.blocking-read': exports0['2'],
        '[method]input-stream.read': exports0['1'],
        '[method]input-stream.subscribe': trampoline4,
        '[method]output-stream.blocking-flush': exports0['5'],
        '[method]output-stream.check-write': exports0['3'],
        '[method]output-stream.subscribe': trampoline5,
        '[method]output-stream.write': exports0['4'],
        '[resource-drop]input-stream': trampoline1,
        '[resource-drop]output-stream': trampoline2,
      },
      'wasi:random/random@0.2.3': {
        'get-random-bytes': exports0['6'],
        'get-random-u64': trampoline9,
      },
      wasi_snapshot_preview1: {
        clock_res_get: exports0['33'],
        clock_time_get: exports0['34'],
        fd_fdstat_get: exports0['35'],
        fd_write: exports0['32'],
      },
    }));
    ({ exports: exports2 } = yield instantiateCore(yield module1, {
      __main_module__: {
        cabi_realloc_adapter: exports1.cabi_realloc_adapter,
      },
      env: {
        memory: exports1.memory,
      },
      'wasi:cli/stderr@0.2.3': {
        'get-stderr': trampoline24,
      },
      'wasi:cli/stdin@0.2.3': {
        'get-stdin': trampoline27,
      },
      'wasi:cli/stdout@0.2.3': {
        'get-stdout': trampoline28,
      },
      'wasi:cli/terminal-input@0.2.3': {
        '[resource-drop]terminal-input': trampoline25,
      },
      'wasi:cli/terminal-output@0.2.3': {
        '[resource-drop]terminal-output': trampoline26,
      },
      'wasi:cli/terminal-stderr@0.2.3': {
        'get-terminal-stderr': exports0['51'],
      },
      'wasi:cli/terminal-stdin@0.2.3': {
        'get-terminal-stdin': exports0['49'],
      },
      'wasi:cli/terminal-stdout@0.2.3': {
        'get-terminal-stdout': exports0['50'],
      },
      'wasi:clocks/monotonic-clock@0.2.3': {
        now: trampoline6,
        resolution: trampoline21,
      },
      'wasi:clocks/wall-clock@0.2.3': {
        now: exports0['37'],
        resolution: exports0['36'],
      },
      'wasi:filesystem/preopens@0.2.3': {
        'get-directories': exports0['48'],
      },
      'wasi:filesystem/types@0.2.3': {
        '[method]descriptor.append-via-stream': exports0['42'],
        '[method]descriptor.get-flags': exports0['38'],
        '[method]descriptor.get-type': exports0['39'],
        '[method]descriptor.stat': exports0['43'],
        '[method]descriptor.write-via-stream': exports0['41'],
        '[resource-drop]descriptor': trampoline23,
        'filesystem-error-code': exports0['40'],
      },
      'wasi:io/error@0.2.3': {
        '[resource-drop]error': trampoline22,
      },
      'wasi:io/streams@0.2.3': {
        '[method]output-stream.blocking-flush': exports0['47'],
        '[method]output-stream.blocking-write-and-flush': exports0['46'],
        '[method]output-stream.check-write': exports0['44'],
        '[method]output-stream.write': exports0['45'],
        '[resource-drop]input-stream': trampoline1,
        '[resource-drop]output-stream': trampoline2,
      },
    }));
    memory0 = exports1.memory;
    realloc0 = exports1.cabi_realloc;
    realloc1 = exports2.cabi_import_realloc;
    ({ exports: exports3 } = yield instantiateCore(yield module3, {
      '': {
        $imports: exports0.$imports,
        '0': trampoline29,
        '1': trampoline30,
        '10': trampoline39,
        '11': trampoline40,
        '12': trampoline41,
        '13': trampoline42,
        '14': trampoline43,
        '15': trampoline44,
        '16': trampoline45,
        '17': trampoline46,
        '18': trampoline47,
        '19': trampoline48,
        '2': trampoline31,
        '20': trampoline49,
        '21': trampoline50,
        '22': trampoline51,
        '23': trampoline52,
        '24': trampoline53,
        '25': trampoline54,
        '26': trampoline55,
        '27': trampoline56,
        '28': trampoline57,
        '29': trampoline58,
        '3': trampoline32,
        '30': trampoline59,
        '31': trampoline60,
        '32': exports2.fd_write,
        '33': exports2.clock_res_get,
        '34': exports2.clock_time_get,
        '35': exports2.fd_fdstat_get,
        '36': trampoline61,
        '37': trampoline62,
        '38': trampoline63,
        '39': trampoline64,
        '4': trampoline33,
        '40': trampoline65,
        '41': trampoline66,
        '42': trampoline67,
        '43': trampoline68,
        '44': trampoline32,
        '45': trampoline33,
        '46': trampoline69,
        '47': trampoline34,
        '48': trampoline70,
        '49': trampoline71,
        '5': trampoline34,
        '50': trampoline72,
        '51': trampoline73,
        '6': trampoline35,
        '7': trampoline36,
        '8': trampoline37,
        '9': trampoline38,
      },
    }));
    postReturn0 = exports1['cabi_post_fastertools:mcp/tool-handler@0.1.9#handle-list-tools'];
    postReturn1 = exports1['cabi_post_fastertools:mcp/tool-handler@0.1.9#handle-call-tool'];
    toolHandler019HandleListTools = exports1['fastertools:mcp/tool-handler@0.1.9#handle-list-tools'];
    toolHandler019HandleCallTool = exports1['fastertools:mcp/tool-handler@0.1.9#handle-call-tool'];
  })();
  let promise, resolve, reject;
  function runNext (value) {
    try {
      let done;
      do {
        ({ value, done } = gen.next(value));
      } while (!(value instanceof Promise) && !done);
      if (done) {
        if (resolve) resolve(value);
        else return value;
      }
      if (!promise) promise = new Promise((_resolve, _reject) => (resolve = _resolve, reject = _reject));
      value.then(runNext, reject);
    }
    catch (e) {
      if (reject) reject(e);
      else throw e;
    }
  }
  const maybeSyncReturn = runNext(null);
  return promise || maybeSyncReturn;
})();

await $init;
const toolHandler019 = {
  handleCallTool: handleCallTool,
  handleListTools: handleListTools,
  
};

export { toolHandler019 as toolHandler, toolHandler019 as 'fastertools:mcp/tool-handler@0.1.9',  }