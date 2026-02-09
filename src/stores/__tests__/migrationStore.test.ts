import { describe, it, expect, beforeEach } from 'vitest';
import { useMigrationStore } from '../migrationStore';

describe('migrationStore', () => {
  beforeEach(() => {
    useMigrationStore.getState().reset();
  });

  describe('default state', () => {
    it('should have default config values', () => {
      const { config } = useMigrationStore.getState();
      expect(config.mode).toBe('Upsert');
      expect(config.conflictResolution).toBe('SourceWins');
      expect(config.batchSize).toBe(1000);
      expect(config.transactionMode).toBe('PerBatch');
      expect(config.retryCount).toBe(3);
      expect(config.autoRollback).toBe(true);
      expect(config.backupBeforeMigrate).toBe(true);
      expect(config.dryRun).toBe(false);
    });

    it('should have idle status', () => {
      expect(useMigrationStore.getState().status).toBe('idle');
    });

    it('should have null source and target connections', () => {
      const state = useMigrationStore.getState();
      expect(state.sourceConnectionId).toBeNull();
      expect(state.targetConnectionId).toBeNull();
    });

    it('should have empty table mappings and transform rules', () => {
      const state = useMigrationStore.getState();
      expect(state.tableMappings).toEqual([]);
      expect(state.transformRules).toEqual([]);
    });

    it('should have wizard step 1', () => {
      expect(useMigrationStore.getState().wizardStep).toBe(1);
    });
  });

  describe('setConfig', () => {
    it('should update partial config', () => {
      useMigrationStore.getState().setConfig({ mode: 'Mirror' });
      expect(useMigrationStore.getState().config.mode).toBe('Mirror');
      // Other fields should remain
      expect(useMigrationStore.getState().config.batchSize).toBe(1000);
    });

    it('should update multiple config fields', () => {
      useMigrationStore.getState().setConfig({
        mode: 'AppendOnly',
        batchSize: 500,
        conflictResolution: 'TargetWins',
      });
      const { config } = useMigrationStore.getState();
      expect(config.mode).toBe('AppendOnly');
      expect(config.batchSize).toBe(500);
      expect(config.conflictResolution).toBe('TargetWins');
    });
  });

  describe('setStatus', () => {
    it('should update migration status', () => {
      useMigrationStore.getState().setStatus('running');
      expect(useMigrationStore.getState().status).toBe('running');
    });
  });

  describe('setWizardStep', () => {
    it('should update wizard step', () => {
      useMigrationStore.getState().setWizardStep(3);
      expect(useMigrationStore.getState().wizardStep).toBe(3);
    });
  });

  describe('source/target connections', () => {
    it('should set source connection', () => {
      useMigrationStore.getState().setSourceConnection('src-1');
      expect(useMigrationStore.getState().sourceConnectionId).toBe('src-1');
    });

    it('should set target connection', () => {
      useMigrationStore.getState().setTargetConnection('tgt-1');
      expect(useMigrationStore.getState().targetConnectionId).toBe('tgt-1');
    });

    it('should allow clearing connections', () => {
      useMigrationStore.getState().setSourceConnection('src-1');
      useMigrationStore.getState().setSourceConnection(null);
      expect(useMigrationStore.getState().sourceConnectionId).toBeNull();
    });
  });

  describe('tableMappings', () => {
    it('should set table mappings', () => {
      const mappings = [
        { id: '1', sourceTable: 'users', targetTable: 'users', included: true, estimatedRows: 100 },
        { id: '2', sourceTable: 'orders', targetTable: 'orders', included: false, estimatedRows: 500 },
      ];
      useMigrationStore.getState().setTableMappings(mappings);
      expect(useMigrationStore.getState().tableMappings).toEqual(mappings);
    });

    it('should update a single table mapping', () => {
      useMigrationStore.getState().setTableMappings([
        { id: '1', sourceTable: 'users', targetTable: '', included: false, estimatedRows: 0 },
      ]);
      useMigrationStore.getState().updateTableMapping('1', {
        targetTable: 'customers',
        included: true,
      });
      const mapping = useMigrationStore.getState().tableMappings[0];
      expect(mapping.targetTable).toBe('customers');
      expect(mapping.included).toBe(true);
      expect(mapping.sourceTable).toBe('users');
    });

    it('should not affect other mappings when updating one', () => {
      useMigrationStore.getState().setTableMappings([
        { id: '1', sourceTable: 'users', targetTable: 'users', included: true, estimatedRows: 100 },
        { id: '2', sourceTable: 'orders', targetTable: 'orders', included: true, estimatedRows: 500 },
      ]);
      useMigrationStore.getState().updateTableMapping('1', { included: false });
      const mappings = useMigrationStore.getState().tableMappings;
      expect(mappings[0].included).toBe(false);
      expect(mappings[1].included).toBe(true);
    });
  });

  describe('transformRules', () => {
    it('should add a transform rule', () => {
      const rule = {
        id: 'r1',
        tableId: 't1',
        sourceColumn: 'name',
        targetColumn: 'full_name',
        ruleType: 'rename' as const,
        config: {},
        order: 0,
      };
      useMigrationStore.getState().addTransformRule(rule);
      expect(useMigrationStore.getState().transformRules).toEqual([rule]);
    });

    it('should remove a transform rule', () => {
      useMigrationStore.getState().addTransformRule({
        id: 'r1',
        tableId: 't1',
        sourceColumn: 'col',
        targetColumn: 'col',
        ruleType: 'rename',
        config: {},
        order: 0,
      });
      useMigrationStore.getState().addTransformRule({
        id: 'r2',
        tableId: 't1',
        sourceColumn: 'col2',
        targetColumn: 'col2',
        ruleType: 'drop_column',
        config: {},
        order: 1,
      });
      useMigrationStore.getState().removeTransformRule('r1');
      const rules = useMigrationStore.getState().transformRules;
      expect(rules).toHaveLength(1);
      expect(rules[0].id).toBe('r2');
    });

    it('should reorder a transform rule', () => {
      useMigrationStore.getState().addTransformRule({
        id: 'r1', tableId: 't1', sourceColumn: 'a', targetColumn: 'a',
        ruleType: 'rename', config: {}, order: 0,
      });
      useMigrationStore.getState().reorderTransformRule('r1', 5);
      expect(useMigrationStore.getState().transformRules[0].order).toBe(5);
    });

    it('should update a transform rule', () => {
      useMigrationStore.getState().addTransformRule({
        id: 'r1', tableId: 't1', sourceColumn: 'a', targetColumn: 'a',
        ruleType: 'rename', config: {}, order: 0,
      });
      useMigrationStore.getState().updateTransformRule('r1', {
        targetColumn: 'b',
        config: { value: 'new_name' },
      });
      const rule = useMigrationStore.getState().transformRules[0];
      expect(rule.targetColumn).toBe('b');
      expect(rule.config.value).toBe('new_name');
    });
  });

  describe('startMigration', () => {
    it('should set status to running and initialize progress', () => {
      useMigrationStore.getState().setTableMappings([
        { id: '1', sourceTable: 'users', targetTable: 'users', included: true, estimatedRows: 100 },
        { id: '2', sourceTable: 'orders', targetTable: 'orders', included: true, estimatedRows: 200 },
        { id: '3', sourceTable: 'logs', targetTable: '', included: false, estimatedRows: 50 },
      ]);

      useMigrationStore.getState().startMigration();
      const state = useMigrationStore.getState();
      expect(state.status).toBe('running');
      expect(state.progress).not.toBeNull();
      expect(state.progress!.totalRows).toBe(300); // 100 + 200 (not 50, excluded)
      expect(state.progress!.processedRows).toBe(0);
      expect(state.progress!.insertedRows).toBe(0);
    });

    it('should create table progress for included tables only', () => {
      useMigrationStore.getState().setTableMappings([
        { id: '1', sourceTable: 'users', targetTable: 'users', included: true, estimatedRows: 100 },
        { id: '2', sourceTable: 'logs', targetTable: '', included: false, estimatedRows: 50 },
      ]);

      useMigrationStore.getState().startMigration();
      const tp = useMigrationStore.getState().tableProgress;
      expect(tp).toHaveLength(1);
      expect(tp[0].tableName).toBe('users');
      expect(tp[0].status).toBe('pending');
    });
  });

  describe('cancelMigration', () => {
    it('should set status to cancelled', () => {
      useMigrationStore.getState().setStatus('running');
      useMigrationStore.getState().cancelMigration();
      expect(useMigrationStore.getState().status).toBe('cancelled');
    });
  });

  describe('reset', () => {
    it('should reset all state to defaults', () => {
      useMigrationStore.getState().setConfig({ mode: 'Mirror', batchSize: 5000 });
      useMigrationStore.getState().setStatus('completed');
      useMigrationStore.getState().setWizardStep(5);
      useMigrationStore.getState().setSourceConnection('src-1');
      useMigrationStore.getState().setTargetConnection('tgt-1');
      useMigrationStore.getState().setTableMappings([
        { id: '1', sourceTable: 'x', targetTable: 'y', included: true, estimatedRows: 10 },
      ]);

      useMigrationStore.getState().reset();
      const state = useMigrationStore.getState();

      expect(state.config.mode).toBe('Upsert');
      expect(state.config.batchSize).toBe(1000);
      expect(state.status).toBe('idle');
      expect(state.wizardStep).toBe(1);
      expect(state.sourceConnectionId).toBeNull();
      expect(state.targetConnectionId).toBeNull();
      expect(state.tableMappings).toEqual([]);
      expect(state.transformRules).toEqual([]);
      expect(state.dryRunResult).toBeNull();
    });
  });

  describe('dryRunResult', () => {
    it('should set and clear dry run result', () => {
      const result = {
        tableSummaries: [{
          tableId: '1', tableName: 'users',
          estimatedRows: 100, estimatedInserts: 50,
          estimatedUpdates: 30, estimatedDeletes: 10, estimatedSkips: 10,
        }],
        warnings: ['test warning'],
        errors: [],
        totalEstimatedTime: 500,
      };
      useMigrationStore.getState().setDryRunResult(result);
      expect(useMigrationStore.getState().dryRunResult).toEqual(result);

      useMigrationStore.getState().setDryRunResult(null);
      expect(useMigrationStore.getState().dryRunResult).toBeNull();
    });
  });

  describe('tableProgress', () => {
    it('should update table progress', () => {
      useMigrationStore.getState().setTableProgress([
        { tableId: 't1', tableName: 'users', status: 'pending', totalRows: 100, processedRows: 0, errors: [] },
      ]);
      useMigrationStore.getState().updateTableProgress('t1', {
        status: 'running',
        processedRows: 50,
      });
      const tp = useMigrationStore.getState().tableProgress[0];
      expect(tp.status).toBe('running');
      expect(tp.processedRows).toBe(50);
      expect(tp.totalRows).toBe(100);
    });
  });

  describe('progress', () => {
    it('should set and clear progress', () => {
      const progress = {
        totalRows: 1000,
        processedRows: 500,
        insertedRows: 300,
        updatedRows: 150,
        deletedRows: 50,
        skippedRows: 0,
        errorCount: 0,
        currentBatch: 5,
        totalBatches: 10,
        elapsedMs: 3000,
      };
      useMigrationStore.getState().setProgress(progress);
      expect(useMigrationStore.getState().progress).toEqual(progress);

      useMigrationStore.getState().setProgress(null);
      expect(useMigrationStore.getState().progress).toBeNull();
    });
  });

  describe('elapsedMs', () => {
    it('should set elapsed milliseconds', () => {
      useMigrationStore.getState().setElapsedMs(5000);
      expect(useMigrationStore.getState().elapsedMs).toBe(5000);
    });
  });
});
