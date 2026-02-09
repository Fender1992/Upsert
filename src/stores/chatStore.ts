import { create } from "zustand";
import {
  saveChatMessageToDb,
  loadChatMessagesFromDb,
  clearChatMessagesInDb,
} from "../lib/tauriCommands";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
  model?: string;
}

export interface OllamaModel {
  name: string;
  size: number;
  modified_at: string;
}

export interface RequiredModelStatus {
  name: string;
  present: boolean;
}

export interface OllamaStatus {
  running: boolean;
  models: OllamaModel[];
  required_models: RequiredModelStatus[];
  all_models_ready: boolean;
}

export interface ModelPullProgress {
  model: string;
  status: string;
  completed: number;
  total: number;
  percent: number;
}

interface ChatState {
  messages: ChatMessage[];
  isStreaming: boolean;
  selectedModel: string | null;
  drawerOpen: boolean;
  ollamaStatus: OllamaStatus | null;
  modelPullProgress: ModelPullProgress | null;
  modelsReady: boolean;
  routingMode: "auto" | "manual";

  setDrawerOpen: (open: boolean) => void;
  toggleDrawer: () => void;
  setSelectedModel: (model: string | null) => void;
  setOllamaStatus: (status: OllamaStatus) => void;
  setStreaming: (streaming: boolean) => void;
  setModelPullProgress: (progress: ModelPullProgress | null) => void;
  setModelsReady: (ready: boolean) => void;
  setRoutingMode: (mode: "auto" | "manual") => void;

  addMessage: (role: ChatMessage["role"], content: string, model?: string) => string;
  appendToLastAssistant: (token: string) => void;
  clearMessages: () => void;
  hydrate: () => Promise<void>;
}

export const useChatStore = create<ChatState>()((set) => ({
  messages: [],
  isStreaming: false,
  selectedModel: null,
  drawerOpen: false,
  ollamaStatus: null,
  modelPullProgress: null,
  modelsReady: false,
  routingMode: "auto",

  setDrawerOpen: (drawerOpen) => set({ drawerOpen }),
  toggleDrawer: () => set((s) => ({ drawerOpen: !s.drawerOpen })),
  setSelectedModel: (selectedModel) => set({ selectedModel }),
  setOllamaStatus: (ollamaStatus) => {
    set((s) => {
      const updates: Partial<ChatState> = {
        ollamaStatus,
        modelsReady: ollamaStatus.all_models_ready,
      };
      // Auto-select first model if none selected
      if (!s.selectedModel && ollamaStatus.models.length > 0) {
        updates.selectedModel = ollamaStatus.models[0].name;
      }
      return updates;
    });
  },
  setStreaming: (isStreaming) => set({ isStreaming }),
  setModelPullProgress: (modelPullProgress) => set({ modelPullProgress }),
  setModelsReady: (modelsReady) => set({ modelsReady }),
  setRoutingMode: (routingMode) => set({ routingMode }),

  addMessage: (role, content, model) => {
    const id = crypto.randomUUID();
    const timestamp = Date.now();
    set((s) => ({
      messages: [...s.messages, { id, role, content, timestamp, model }],
    }));
    // Persist to DB (fire-and-forget)
    saveChatMessageToDb({
      id,
      role,
      content,
      model: model ?? null,
      timestamp,
    }).catch((e) => console.error("Failed to persist chat message:", e));
    return id;
  },

  appendToLastAssistant: (token) =>
    set((s) => {
      const msgs = [...s.messages];
      const last = msgs[msgs.length - 1];
      if (last && last.role === "assistant") {
        msgs[msgs.length - 1] = { ...last, content: last.content + token };
      }
      return { messages: msgs };
    }),

  clearMessages: () => {
    set({ messages: [] });
    clearChatMessagesInDb().catch((e) =>
      console.error("Failed to clear chat messages in DB:", e),
    );
  },

  hydrate: async () => {
    try {
      const rows = await loadChatMessagesFromDb();
      if (rows.length > 0) {
        const messages: ChatMessage[] = rows.map((r) => ({
          id: r.id,
          role: r.role as ChatMessage["role"],
          content: r.content,
          model: r.model ?? undefined,
          timestamp: r.timestamp,
        }));
        set({ messages });
      }
    } catch (e) {
      console.error("Failed to hydrate chat messages:", e);
    }
  },
}));
