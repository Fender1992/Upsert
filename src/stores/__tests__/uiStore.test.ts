import { describe, it, expect, beforeEach } from 'vitest';
import { useUiStore } from '../uiStore';

describe('uiStore', () => {
  beforeEach(() => {
    useUiStore.setState({
      theme: 'system',
      sidebarCollapsed: false,
      bottomPanelHeight: 200,
      bottomPanelVisible: true,
      tabs: [],
      activeTabId: null,
      commandPaletteOpen: false,
      notifications: [],
      outputLog: [],
    });
  });

  describe('addTab', () => {
    it('should add a tab with generated id', () => {
      useUiStore.getState().addTab({
        title: 'Comparison 1',
        type: 'comparison',
      });

      const { tabs } = useUiStore.getState();
      expect(tabs).toHaveLength(1);
      expect(tabs[0].title).toBe('Comparison 1');
      expect(tabs[0].type).toBe('comparison');
      expect(tabs[0].id).toBeDefined();
    });

    it('should set the new tab as active', () => {
      useUiStore.getState().addTab({
        title: 'Comparison 1',
        type: 'comparison',
      });

      const { tabs, activeTabId } = useUiStore.getState();
      expect(activeTabId).toBe(tabs[0].id);
    });

    it('should return the new tab id', () => {
      const id = useUiStore.getState().addTab({
        title: 'Migration 1',
        type: 'migration',
      });

      expect(id).toBeDefined();
      expect(typeof id).toBe('string');
      expect(useUiStore.getState().tabs[0].id).toBe(id);
    });

    it('should add multiple tabs', () => {
      const { addTab } = useUiStore.getState();
      addTab({ title: 'Tab 1', type: 'comparison' });
      addTab({ title: 'Tab 2', type: 'migration' });
      addTab({ title: 'Tab 3', type: 'query' });

      expect(useUiStore.getState().tabs).toHaveLength(3);
    });
  });

  describe('removeTab', () => {
    it('should remove a tab by id', () => {
      const id = useUiStore.getState().addTab({
        title: 'To Remove',
        type: 'comparison',
      });

      useUiStore.getState().removeTab(id);
      expect(useUiStore.getState().tabs).toHaveLength(0);
    });

    it('should set activeTabId to the last remaining tab when removing the active tab', () => {
      useUiStore.getState().addTab({ title: 'Tab 1', type: 'comparison' });
      const id2 = useUiStore.getState().addTab({ title: 'Tab 2', type: 'migration' });

      // Tab 2 is active (last added)
      expect(useUiStore.getState().activeTabId).toBe(id2);

      useUiStore.getState().removeTab(id2);

      const state = useUiStore.getState();
      expect(state.tabs).toHaveLength(1);
      expect(state.activeTabId).toBe(state.tabs[0].id);
    });

    it('should set activeTabId to null when removing the last tab', () => {
      const id = useUiStore.getState().addTab({
        title: 'Only Tab',
        type: 'comparison',
      });

      useUiStore.getState().removeTab(id);
      expect(useUiStore.getState().activeTabId).toBeNull();
    });

    it('should not change activeTabId when removing a non-active tab', () => {
      const id1 = useUiStore.getState().addTab({ title: 'Tab 1', type: 'comparison' });
      useUiStore.getState().addTab({ title: 'Tab 2', type: 'migration' });

      // Set Tab 2 as active (it already is from addTab), now switch to Tab 1
      useUiStore.getState().setActiveTab(id1);

      const tab2Id = useUiStore.getState().tabs[1].id;
      useUiStore.getState().removeTab(tab2Id);

      expect(useUiStore.getState().activeTabId).toBe(id1);
    });
  });

  describe('setActiveTab', () => {
    it('should set the active tab id', () => {
      const id = useUiStore.getState().addTab({
        title: 'Test',
        type: 'comparison',
      });

      useUiStore.getState().setActiveTab(id);
      expect(useUiStore.getState().activeTabId).toBe(id);
    });

    it('should allow setting to null', () => {
      useUiStore.getState().addTab({ title: 'Test', type: 'comparison' });
      useUiStore.getState().setActiveTab(null);

      expect(useUiStore.getState().activeTabId).toBeNull();
    });
  });

  describe('toggleSidebar', () => {
    it('should toggle sidebar from collapsed false to true', () => {
      expect(useUiStore.getState().sidebarCollapsed).toBe(false);

      useUiStore.getState().toggleSidebar();
      expect(useUiStore.getState().sidebarCollapsed).toBe(true);
    });

    it('should toggle sidebar from collapsed true to false', () => {
      useUiStore.setState({ sidebarCollapsed: true });

      useUiStore.getState().toggleSidebar();
      expect(useUiStore.getState().sidebarCollapsed).toBe(false);
    });

    it('should toggle back and forth', () => {
      useUiStore.getState().toggleSidebar();
      expect(useUiStore.getState().sidebarCollapsed).toBe(true);

      useUiStore.getState().toggleSidebar();
      expect(useUiStore.getState().sidebarCollapsed).toBe(false);
    });
  });

  describe('theme changes', () => {
    it('should default to system theme', () => {
      expect(useUiStore.getState().theme).toBe('system');
    });

    it('should set theme to light', () => {
      useUiStore.getState().setTheme('light');
      expect(useUiStore.getState().theme).toBe('light');
    });

    it('should set theme to dark', () => {
      useUiStore.getState().setTheme('dark');
      expect(useUiStore.getState().theme).toBe('dark');
    });

    it('should set theme back to system', () => {
      useUiStore.getState().setTheme('dark');
      useUiStore.getState().setTheme('system');
      expect(useUiStore.getState().theme).toBe('system');
    });
  });
});
