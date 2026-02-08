import { describe, it, expect, beforeEach } from 'vitest';
import { useSettingsStore } from '../settingsStore';

describe('settingsStore', () => {
  beforeEach(() => {
    useSettingsStore.setState({
      defaultBatchSize: 1000,
      defaultTimeout: 30,
      defaultReadOnly: true,
      autoSaveJobs: true,
      confirmDestructiveOps: true,
      maxRecentConnections: 10,
      hasCompletedOnboarding: false,
    });
  });

  describe('setSetting', () => {
    it('should update defaultBatchSize', () => {
      useSettingsStore.getState().setSetting('defaultBatchSize', 5000);
      expect(useSettingsStore.getState().defaultBatchSize).toBe(5000);
    });

    it('should update defaultTimeout', () => {
      useSettingsStore.getState().setSetting('defaultTimeout', 60);
      expect(useSettingsStore.getState().defaultTimeout).toBe(60);
    });

    it('should update defaultReadOnly', () => {
      useSettingsStore.getState().setSetting('defaultReadOnly', false);
      expect(useSettingsStore.getState().defaultReadOnly).toBe(false);
    });

    it('should update autoSaveJobs', () => {
      useSettingsStore.getState().setSetting('autoSaveJobs', false);
      expect(useSettingsStore.getState().autoSaveJobs).toBe(false);
    });

    it('should update confirmDestructiveOps', () => {
      useSettingsStore.getState().setSetting('confirmDestructiveOps', false);
      expect(useSettingsStore.getState().confirmDestructiveOps).toBe(false);
    });

    it('should update maxRecentConnections', () => {
      useSettingsStore.getState().setSetting('maxRecentConnections', 25);
      expect(useSettingsStore.getState().maxRecentConnections).toBe(25);
    });

    it('should not affect other settings when updating one', () => {
      useSettingsStore.getState().setSetting('defaultBatchSize', 2000);

      const state = useSettingsStore.getState();
      expect(state.defaultBatchSize).toBe(2000);
      expect(state.defaultTimeout).toBe(30);
      expect(state.defaultReadOnly).toBe(true);
      expect(state.autoSaveJobs).toBe(true);
    });
  });

  describe('setHasCompletedOnboarding', () => {
    it('should default to false', () => {
      expect(useSettingsStore.getState().hasCompletedOnboarding).toBe(false);
    });

    it('should set hasCompletedOnboarding to true', () => {
      useSettingsStore.getState().setHasCompletedOnboarding(true);
      expect(useSettingsStore.getState().hasCompletedOnboarding).toBe(true);
    });

    it('should set hasCompletedOnboarding back to false', () => {
      useSettingsStore.getState().setHasCompletedOnboarding(true);
      useSettingsStore.getState().setHasCompletedOnboarding(false);
      expect(useSettingsStore.getState().hasCompletedOnboarding).toBe(false);
    });
  });
});
