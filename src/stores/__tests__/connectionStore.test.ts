import { describe, it, expect, beforeEach } from 'vitest';
import { useConnectionStore } from '../connectionStore';

describe('connectionStore', () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useConnectionStore.setState({
      connections: [],
      activeConnectionId: null,
      isLoading: false,
      error: null,
    });
  });

  describe('addConnection', () => {
    it('should add a connection with generated id and disconnected status', () => {
      useConnectionStore.getState().addConnection({
        name: 'Test DB',
        engine: 'PostgreSql',
        host: 'localhost',
        port: 5432,
        database: 'testdb',
        username: 'admin',
        readOnly: false,
      });

      const { connections } = useConnectionStore.getState();
      expect(connections).toHaveLength(1);
      expect(connections[0].name).toBe('Test DB');
      expect(connections[0].engine).toBe('PostgreSql');
      expect(connections[0].host).toBe('localhost');
      expect(connections[0].port).toBe(5432);
      expect(connections[0].status).toBe('disconnected');
      expect(connections[0].id).toBeDefined();
    });

    it('should add multiple connections', () => {
      const { addConnection } = useConnectionStore.getState();

      addConnection({
        name: 'DB One',
        engine: 'SqlServer',
        host: 'server1',
        readOnly: true,
      });
      addConnection({
        name: 'DB Two',
        engine: 'MySql',
        host: 'server2',
        readOnly: false,
      });

      const { connections } = useConnectionStore.getState();
      expect(connections).toHaveLength(2);
      expect(connections[0].name).toBe('DB One');
      expect(connections[1].name).toBe('DB Two');
    });

    it('should assign unique ids to each connection', () => {
      const { addConnection } = useConnectionStore.getState();

      addConnection({ name: 'A', engine: 'Sqlite', readOnly: true });
      addConnection({ name: 'B', engine: 'Sqlite', readOnly: true });

      const { connections } = useConnectionStore.getState();
      expect(connections[0].id).not.toBe(connections[1].id);
    });
  });

  describe('removeConnection', () => {
    it('should remove a connection by id', () => {
      useConnectionStore.getState().addConnection({
        name: 'To Remove',
        engine: 'PostgreSql',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().removeConnection(id);

      expect(useConnectionStore.getState().connections).toHaveLength(0);
    });

    it('should clear activeConnectionId when removing the active connection', () => {
      useConnectionStore.getState().addConnection({
        name: 'Active',
        engine: 'PostgreSql',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().setActiveConnection(id);
      expect(useConnectionStore.getState().activeConnectionId).toBe(id);

      useConnectionStore.getState().removeConnection(id);
      expect(useConnectionStore.getState().activeConnectionId).toBeNull();
    });

    it('should not change activeConnectionId when removing a non-active connection', () => {
      const { addConnection } = useConnectionStore.getState();
      addConnection({ name: 'Active', engine: 'PostgreSql', readOnly: false });
      addConnection({ name: 'Other', engine: 'MySql', readOnly: false });

      const connections = useConnectionStore.getState().connections;
      const activeId = connections[0].id;
      const otherId = connections[1].id;

      useConnectionStore.getState().setActiveConnection(activeId);
      useConnectionStore.getState().removeConnection(otherId);

      expect(useConnectionStore.getState().activeConnectionId).toBe(activeId);
      expect(useConnectionStore.getState().connections).toHaveLength(1);
    });
  });

  describe('updateConnection', () => {
    it('should update specific fields on a connection', () => {
      useConnectionStore.getState().addConnection({
        name: 'Original',
        engine: 'PostgreSql',
        host: 'localhost',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().updateConnection(id, {
        name: 'Updated',
        host: 'remotehost',
      });

      const conn = useConnectionStore.getState().connections[0];
      expect(conn.name).toBe('Updated');
      expect(conn.host).toBe('remotehost');
      expect(conn.engine).toBe('PostgreSql');
    });

    it('should not affect other connections', () => {
      const { addConnection } = useConnectionStore.getState();
      addConnection({ name: 'First', engine: 'PostgreSql', readOnly: false });
      addConnection({ name: 'Second', engine: 'MySql', readOnly: false });

      const connections = useConnectionStore.getState().connections;
      useConnectionStore.getState().updateConnection(connections[0].id, {
        name: 'First Updated',
      });

      const updated = useConnectionStore.getState().connections;
      expect(updated[0].name).toBe('First Updated');
      expect(updated[1].name).toBe('Second');
    });
  });

  describe('setActiveConnection', () => {
    it('should set the active connection id', () => {
      useConnectionStore.getState().addConnection({
        name: 'Test',
        engine: 'PostgreSql',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().setActiveConnection(id);

      expect(useConnectionStore.getState().activeConnectionId).toBe(id);
    });

    it('should allow setting active connection to null', () => {
      useConnectionStore.getState().addConnection({
        name: 'Test',
        engine: 'PostgreSql',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().setActiveConnection(id);
      useConnectionStore.getState().setActiveConnection(null);

      expect(useConnectionStore.getState().activeConnectionId).toBeNull();
    });
  });

  describe('setConnectionStatus', () => {
    it('should update connection status', () => {
      useConnectionStore.getState().addConnection({
        name: 'Test',
        engine: 'PostgreSql',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().setConnectionStatus(id, 'connected');

      expect(useConnectionStore.getState().connections[0].status).toBe('connected');
    });

    it('should set error when status is error', () => {
      useConnectionStore.getState().addConnection({
        name: 'Test',
        engine: 'PostgreSql',
        readOnly: false,
      });

      const id = useConnectionStore.getState().connections[0].id;
      useConnectionStore.getState().setConnectionStatus(id, 'error', 'Connection refused');

      const conn = useConnectionStore.getState().connections[0];
      expect(conn.status).toBe('error');
      expect(conn.error).toBe('Connection refused');
    });
  });
});
