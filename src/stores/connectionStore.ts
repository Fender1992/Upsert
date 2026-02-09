import { create } from "zustand";

export type DatabaseEngine =
  | "SqlServer"
  | "PostgreSql"
  | "MySql"
  | "Sqlite"
  | "Oracle"
  | "MongoDb"
  | "CosmosDb";

export interface ConnectionProfile {
  id: string;
  name: string;
  engine: DatabaseEngine;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  password?: string;
  filePath?: string;
  readOnly: boolean;
  status: "disconnected" | "connecting" | "connected" | "error";
  error?: string;
}

interface ConnectionState {
  connections: ConnectionProfile[];
  activeConnectionId: string | null;
  isLoading: boolean;
  error: string | null;

  addConnection: (profile: Omit<ConnectionProfile, "id" | "status">) => void;
  removeConnection: (id: string) => void;
  updateConnection: (id: string, updates: Partial<ConnectionProfile>) => void;
  setActiveConnection: (id: string | null) => void;
  setConnectionStatus: (
    id: string,
    status: ConnectionProfile["status"],
    error?: string,
  ) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useConnectionStore = create<ConnectionState>()((set) => ({
  connections: [],
  activeConnectionId: null,
  isLoading: false,
  error: null,

  addConnection: (profile) =>
    set((state) => ({
      connections: [
        ...state.connections,
        {
          ...profile,
          id: crypto.randomUUID(),
          status: "disconnected" as const,
        },
      ],
    })),

  removeConnection: (id) =>
    set((state) => ({
      connections: state.connections.filter((c) => c.id !== id),
      activeConnectionId:
        state.activeConnectionId === id ? null : state.activeConnectionId,
    })),

  updateConnection: (id, updates) =>
    set((state) => ({
      connections: state.connections.map((c) =>
        c.id === id ? { ...c, ...updates } : c,
      ),
    })),

  setActiveConnection: (id) => set({ activeConnectionId: id }),

  setConnectionStatus: (id, status, error) =>
    set((state) => ({
      connections: state.connections.map((c) =>
        c.id === id ? { ...c, status, error } : c,
      ),
    })),

  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),
}));
