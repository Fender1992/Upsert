import { create } from "zustand";
import {
  saveConnectionProfile,
  getConnectionProfiles,
  deleteConnectionProfile,
  type ConnectionProfileDto,
} from "../lib/tauriCommands";

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
  hydrate: () => Promise<void>;
}

function toDto(profile: ConnectionProfile): ConnectionProfileDto {
  return {
    id: profile.id,
    name: profile.name,
    engine: profile.engine,
    host: profile.host ?? null,
    port: profile.port ?? null,
    databaseName: profile.database ?? null,
    username: profile.username ?? null,
    filePath: profile.filePath ?? null,
    readOnly: profile.readOnly,
    credentialKey: null,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
  };
}

function fromDto(dto: ConnectionProfileDto): ConnectionProfile {
  return {
    id: dto.id,
    name: dto.name,
    engine: dto.engine as DatabaseEngine,
    host: dto.host ?? undefined,
    port: dto.port ?? undefined,
    database: dto.databaseName ?? undefined,
    username: dto.username ?? undefined,
    filePath: dto.filePath ?? undefined,
    readOnly: dto.readOnly,
    status: "disconnected",
  };
}

export const useConnectionStore = create<ConnectionState>()((set) => ({
  connections: [],
  activeConnectionId: null,
  isLoading: false,
  error: null,

  addConnection: (profile) => {
    const id = crypto.randomUUID();
    const newProfile: ConnectionProfile = {
      ...profile,
      id,
      status: "disconnected" as const,
    };
    set((state) => ({
      connections: [...state.connections, newProfile],
    }));
    // Persist (fire-and-forget)
    saveConnectionProfile(toDto(newProfile)).catch((e) =>
      console.error("Failed to persist connection:", e),
    );
  },

  removeConnection: (id) => {
    set((state) => ({
      connections: state.connections.filter((c) => c.id !== id),
      activeConnectionId:
        state.activeConnectionId === id ? null : state.activeConnectionId,
    }));
    deleteConnectionProfile(id).catch((e) =>
      console.error("Failed to delete persisted connection:", e),
    );
  },

  updateConnection: (id, updates) => {
    set((state) => {
      const updated = state.connections.map((c) =>
        c.id === id ? { ...c, ...updates } : c,
      );
      // Persist the updated profile
      const profile = updated.find((c) => c.id === id);
      if (profile) {
        saveConnectionProfile(toDto(profile)).catch((e) =>
          console.error("Failed to persist connection update:", e),
        );
      }
      return { connections: updated };
    });
  },

  setActiveConnection: (id) => set({ activeConnectionId: id }),

  setConnectionStatus: (id, status, error) =>
    set((state) => ({
      connections: state.connections.map((c) =>
        c.id === id ? { ...c, status, error } : c,
      ),
    })),

  setLoading: (isLoading) => set({ isLoading }),
  setError: (error) => set({ error }),

  hydrate: async () => {
    try {
      const profiles = await getConnectionProfiles();
      const connections = profiles.map(fromDto);
      set((state) => {
        // Merge: keep any already-loaded connections that aren't in DB
        const existingIds = new Set(connections.map((c) => c.id));
        const kept = state.connections.filter((c) => !existingIds.has(c.id));
        return { connections: [...connections, ...kept] };
      });
    } catch (e) {
      console.error("Failed to hydrate connections:", e);
    }
  },
}));
