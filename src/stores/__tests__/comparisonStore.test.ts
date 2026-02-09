import { describe, it, expect, beforeEach } from 'vitest';
import { useComparisonStore } from '../comparisonStore';

describe('comparisonStore', () => {
  beforeEach(() => {
    useComparisonStore.getState().reset();
  });

  describe('default state', () => {
    it('should have null schema and data diffs', () => {
      const state = useComparisonStore.getState();
      expect(state.schemaDiff).toBeNull();
      expect(state.dataDiff).toBeNull();
    });

    it('should not be comparing', () => {
      expect(useComparisonStore.getState().isComparing).toBe(false);
    });

    it('should have zero progress', () => {
      expect(useComparisonStore.getState().progress).toBe(0);
    });

    it('should have null error', () => {
      expect(useComparisonStore.getState().error).toBeNull();
    });
  });

  describe('setSchemaDiff', () => {
    it('should set schema diff result', () => {
      const diff = {
        sourceDatabase: 'source_db',
        targetDatabase: 'target_db',
        changes: [
          {
            objectType: 'Table',
            objectName: 'users',
            changeType: 'Added' as const,
            details: [],
          },
        ],
        summary: { additions: 1, removals: 0, modifications: 0, unchanged: 5 },
      };
      useComparisonStore.getState().setSchemaDiff(diff);
      expect(useComparisonStore.getState().schemaDiff).toEqual(diff);
    });

    it('should clear schema diff with null', () => {
      useComparisonStore.getState().setSchemaDiff({
        sourceDatabase: 'a', targetDatabase: 'b',
        changes: [], summary: { additions: 0, removals: 0, modifications: 0, unchanged: 0 },
      });
      useComparisonStore.getState().setSchemaDiff(null);
      expect(useComparisonStore.getState().schemaDiff).toBeNull();
    });
  });

  describe('setDataDiff', () => {
    it('should set data diff result', () => {
      const diff = {
        sourceTable: 'users',
        targetTable: 'users',
        matchedRows: 100,
        insertedCount: 10,
        updatedCount: 5,
        deletedCount: 2,
        errorCount: 0,
      };
      useComparisonStore.getState().setDataDiff(diff);
      expect(useComparisonStore.getState().dataDiff).toEqual(diff);
    });

    it('should clear data diff with null', () => {
      useComparisonStore.getState().setDataDiff({
        sourceTable: 'a', targetTable: 'b',
        matchedRows: 0, insertedCount: 0, updatedCount: 0, deletedCount: 0, errorCount: 0,
      });
      useComparisonStore.getState().setDataDiff(null);
      expect(useComparisonStore.getState().dataDiff).toBeNull();
    });
  });

  describe('setComparing', () => {
    it('should set comparing to true', () => {
      useComparisonStore.getState().setComparing(true);
      expect(useComparisonStore.getState().isComparing).toBe(true);
    });

    it('should set comparing back to false', () => {
      useComparisonStore.getState().setComparing(true);
      useComparisonStore.getState().setComparing(false);
      expect(useComparisonStore.getState().isComparing).toBe(false);
    });
  });

  describe('setProgress', () => {
    it('should set progress percentage', () => {
      useComparisonStore.getState().setProgress(75);
      expect(useComparisonStore.getState().progress).toBe(75);
    });

    it('should allow setting to 100', () => {
      useComparisonStore.getState().setProgress(100);
      expect(useComparisonStore.getState().progress).toBe(100);
    });
  });

  describe('setError', () => {
    it('should set error message', () => {
      useComparisonStore.getState().setError('Connection failed');
      expect(useComparisonStore.getState().error).toBe('Connection failed');
    });

    it('should clear error with null', () => {
      useComparisonStore.getState().setError('An error');
      useComparisonStore.getState().setError(null);
      expect(useComparisonStore.getState().error).toBeNull();
    });
  });

  describe('reset', () => {
    it('should reset all state to defaults', () => {
      useComparisonStore.getState().setSchemaDiff({
        sourceDatabase: 'a', targetDatabase: 'b',
        changes: [], summary: { additions: 1, removals: 0, modifications: 0, unchanged: 0 },
      });
      useComparisonStore.getState().setDataDiff({
        sourceTable: 'x', targetTable: 'y',
        matchedRows: 10, insertedCount: 2, updatedCount: 1, deletedCount: 0, errorCount: 0,
      });
      useComparisonStore.getState().setComparing(true);
      useComparisonStore.getState().setProgress(50);
      useComparisonStore.getState().setError('something');

      useComparisonStore.getState().reset();
      const state = useComparisonStore.getState();
      expect(state.schemaDiff).toBeNull();
      expect(state.dataDiff).toBeNull();
      expect(state.isComparing).toBe(false);
      expect(state.progress).toBe(0);
      expect(state.error).toBeNull();
    });
  });
});
