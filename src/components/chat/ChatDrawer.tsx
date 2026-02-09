import { useState, useRef, useEffect, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { useChatStore, type ModelPullProgress } from "../../stores/chatStore";
import {
  checkOllamaStatus,
  sendChatMessage,
  type ChatMessageDto,
} from "../../lib/tauriCommands";
import { buildChatContext } from "../../lib/chatContext";
import { getModelForQuery } from "../../lib/queryRouter";

export default function ChatDrawer() {
  const {
    messages,
    isStreaming,
    selectedModel,
    drawerOpen,
    ollamaStatus,
    modelPullProgress,
    routingMode,
    setDrawerOpen,
    setSelectedModel,
    setOllamaStatus,
    setStreaming,
    setModelPullProgress,
    setModelsReady,
    setRoutingMode,
    addMessage,
    appendToLastAssistant,
    clearMessages,
  } = useChatStore();

  const [input, setInput] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Listen for model pull progress events
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<ModelPullProgress>("model-pull-progress", (event) => {
      setModelPullProgress(event.payload);
      if (event.payload.status === "success") {
        // Refresh status after a model finishes pulling
        checkOllamaStatus()
          .then(setOllamaStatus)
          .catch(() => {});
      }
    }).then((fn) => {
      unlisten = fn;
    });

    let unlistenReady: (() => void) | undefined;
    listen<boolean>("models-ready", () => {
      setModelsReady(true);
      setModelPullProgress(null);
      checkOllamaStatus()
        .then(setOllamaStatus)
        .catch(() => {});
    }).then((fn) => {
      unlistenReady = fn;
    });

    return () => {
      unlisten?.();
      unlistenReady?.();
    };
  }, [setModelPullProgress, setModelsReady, setOllamaStatus]);

  // Check Ollama status on open
  useEffect(() => {
    if (drawerOpen) {
      checkOllamaStatus()
        .then(setOllamaStatus)
        .catch(() =>
          setOllamaStatus({ running: false, models: [], required_models: [], all_models_ready: false }),
        );
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [drawerOpen, setOllamaStatus]);

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const handleSend = useCallback(async () => {
    const text = input.trim();
    if (!text || isStreaming) return;

    // Determine model: auto-route or manual
    const model = routingMode === "auto"
      ? getModelForQuery(text)
      : selectedModel;

    if (!model) return;

    setInput("");
    addMessage("user", text);

    // Build messages array with system context (async — fetches live schemas)
    const systemContext = await buildChatContext(text);
    const allMessages: ChatMessageDto[] = [
      { role: "system", content: systemContext },
      ...messages.map((m) => ({ role: m.role, content: m.content })),
      { role: "user", content: text },
    ];

    const requestId = crypto.randomUUID();

    // Create placeholder assistant message with model tag
    addMessage("assistant", "", model);
    setStreaming(true);

    // Listen for streaming tokens
    const unlisten = await listen<string>(
      `chat-stream-${requestId}`,
      (event) => {
        appendToLastAssistant(event.payload);
      },
    );

    try {
      await sendChatMessage(model, allMessages, requestId);
    } catch (err) {
      appendToLastAssistant(
        `\n\n**Error:** ${err instanceof Error ? err.message : String(err)}`,
      );
    } finally {
      unlisten();
      setStreaming(false);
    }
  }, [
    input,
    isStreaming,
    selectedModel,
    routingMode,
    messages,
    addMessage,
    appendToLastAssistant,
    setStreaming,
  ]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  if (!drawerOpen) return null;

  const models = ollamaStatus?.models ?? [];
  const isRunning = ollamaStatus?.running ?? false;
  const isPulling = modelPullProgress && modelPullProgress.status !== "success" && !modelPullProgress.status.startsWith("error");

  return (
    <div className="fixed inset-y-0 right-0 z-30 flex w-96 flex-col border-l border-neutral-300 bg-white shadow-xl dark:border-neutral-700 dark:bg-neutral-900">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-neutral-200 px-4 py-2 dark:border-neutral-700">
        <div className="flex items-center gap-2">
          <svg className="h-4 w-4 text-purple-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
            <path strokeLinecap="round" strokeLinejoin="round" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
          </svg>
          <span className="text-sm font-semibold">AI Chat</span>
          {!isRunning && (
            <span className="rounded bg-red-100 px-1.5 py-0.5 text-[10px] font-medium text-red-700 dark:bg-red-900/40 dark:text-red-400">
              Offline
            </span>
          )}
          {isRunning && isPulling && (
            <span className="rounded bg-yellow-100 px-1.5 py-0.5 text-[10px] font-medium text-yellow-700 dark:bg-yellow-900/40 dark:text-yellow-400">
              Setup
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={clearMessages}
            className="rounded p-1 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:hover:bg-neutral-800 dark:hover:text-neutral-300"
            title="Clear chat"
          >
            <svg className="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
            </svg>
          </button>
          <button
            onClick={() => setDrawerOpen(false)}
            className="rounded p-1 text-neutral-500 hover:bg-neutral-100 hover:text-neutral-700 dark:hover:bg-neutral-800 dark:hover:text-neutral-300"
            title="Close (Ctrl+L)"
          >
            <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>
      </div>

      {/* Model selector + routing mode */}
      {isRunning && models.length > 0 && (
        <div className="flex items-center gap-2 border-b border-neutral-200 px-4 py-2 dark:border-neutral-700">
          <select
            value={routingMode}
            onChange={(e) => setRoutingMode(e.target.value as "auto" | "manual")}
            className="rounded border border-neutral-300 bg-transparent px-1.5 py-1 text-xs dark:border-neutral-600 dark:text-neutral-200"
          >
            <option value="auto">Auto</option>
            <option value="manual">Manual</option>
          </select>
          {routingMode === "manual" ? (
            <select
              value={selectedModel ?? ""}
              onChange={(e) => setSelectedModel(e.target.value)}
              className="flex-1 rounded border border-neutral-300 bg-transparent px-2 py-1 text-xs dark:border-neutral-600 dark:text-neutral-200"
            >
              {models.map((m) => (
                <option key={m.name} value={m.name}>
                  {m.name}
                </option>
              ))}
            </select>
          ) : (
            <span className="text-[10px] text-neutral-400 dark:text-neutral-500">
              Routes: simple → tinyllama, complex → llama3.2:3b
            </span>
          )}
        </div>
      )}

      {/* Model pull progress banner */}
      {isPulling && modelPullProgress && (
        <div className="border-b border-neutral-200 bg-blue-50 px-4 py-2 dark:border-neutral-700 dark:bg-blue-950/30">
          <div className="flex items-center justify-between text-xs">
            <span className="font-medium text-blue-700 dark:text-blue-400">
              Downloading {modelPullProgress.model}
            </span>
            <span className="text-blue-600 dark:text-blue-400">
              {modelPullProgress.percent > 0 ? `${modelPullProgress.percent.toFixed(0)}%` : modelPullProgress.status}
            </span>
          </div>
          {modelPullProgress.total > 0 && (
            <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-blue-200 dark:bg-blue-900">
              <div
                className="h-full rounded-full bg-blue-600 transition-all dark:bg-blue-400"
                style={{ width: `${Math.min(modelPullProgress.percent, 100)}%` }}
              />
            </div>
          )}
        </div>
      )}

      {/* Messages */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-3">
        {!isRunning ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <svg className="h-10 w-10 text-neutral-300 dark:text-neutral-600 mb-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
            </svg>
            <p className="text-sm font-medium text-neutral-500 dark:text-neutral-400">
              Ollama is starting up...
            </p>
            <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
              The AI assistant will be ready shortly.
            </p>
            <button
              onClick={() =>
                checkOllamaStatus()
                  .then(setOllamaStatus)
                  .catch(() =>
                    setOllamaStatus({ running: false, models: [], required_models: [], all_models_ready: false }),
                  )
              }
              className="mt-3 rounded border border-neutral-300 px-3 py-1 text-xs text-neutral-600 hover:bg-neutral-100 dark:border-neutral-600 dark:text-neutral-400 dark:hover:bg-neutral-800"
            >
              Retry Connection
            </button>
          </div>
        ) : messages.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-full text-center">
            <svg className="h-10 w-10 text-neutral-300 dark:text-neutral-600 mb-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.5}>
              <path strokeLinecap="round" strokeLinejoin="round" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
            </svg>
            <p className="text-sm font-medium text-neutral-500 dark:text-neutral-400">
              Ask me anything about databases
            </p>
            <p className="mt-1 text-xs text-neutral-400 dark:text-neutral-500">
              I can help with SQL, schema design, migrations, and more.
            </p>
          </div>
        ) : (
          messages
            .filter((m) => m.role !== "system")
            .map((msg) => (
              <div
                key={msg.id}
                className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
              >
                <div className="max-w-[85%]">
                  {msg.role === "assistant" && msg.model && (
                    <span className="mb-0.5 block text-[9px] font-medium uppercase tracking-wide text-neutral-400 dark:text-neutral-500">
                      {msg.model}
                    </span>
                  )}
                  <div
                    className={`rounded-lg px-3 py-2 text-sm whitespace-pre-wrap ${
                      msg.role === "user"
                        ? "bg-blue-600 text-white"
                        : "bg-neutral-100 text-neutral-800 dark:bg-neutral-800 dark:text-neutral-200"
                    }`}
                  >
                    {msg.content || (
                      <span className="inline-flex items-center gap-1 text-neutral-400">
                        <span className="animate-pulse">Thinking...</span>
                      </span>
                    )}
                  </div>
                </div>
              </div>
            ))
        )}
        <div ref={messagesEndRef} />
      </div>

      {/* Input */}
      {isRunning && (
        <div className="border-t border-neutral-200 px-4 py-3 dark:border-neutral-700">
          <div className="flex gap-2">
            <textarea
              ref={inputRef}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask about databases..."
              rows={1}
              className="flex-1 resize-none rounded border border-neutral-300 bg-transparent px-3 py-2 text-sm outline-none focus:border-blue-500 dark:border-neutral-600 dark:text-neutral-100 dark:focus:border-blue-400"
              disabled={isStreaming}
            />
            <button
              onClick={handleSend}
              disabled={isStreaming || !input.trim()}
              className="rounded bg-blue-600 px-3 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
              title="Send (Enter)"
            >
              <svg className="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2}>
                <path strokeLinecap="round" strokeLinejoin="round" d="M12 19V5m0 0l-7 7m7-7l7 7" />
              </svg>
            </button>
          </div>
          <p className="mt-1 text-[10px] text-neutral-400">
            Shift+Enter for new line
            {routingMode === "auto" && " · Auto-routing enabled"}
          </p>
        </div>
      )}
    </div>
  );
}
