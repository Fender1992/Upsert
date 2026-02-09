import { useConnectionStore } from "../stores/connectionStore";
import { useUiStore } from "../stores/uiStore";
import { useMigrationStore } from "../stores/migrationStore";
import { useComparisonStore } from "../stores/comparisonStore";
import { searchContext } from "./tauriCommands";

/**
 * Build a focused system-prompt context string using RAG retrieval.
 * Embeds the user query, retrieves top-5 relevant context chunks,
 * and combines with lightweight inline state.
 */
export async function buildChatContext(userQuery: string): Promise<string> {
  const { connections, activeConnectionId } = useConnectionStore.getState();
  const { tabs, activeTabId } = useUiStore.getState();
  const migration = useMigrationStore.getState();
  const comparison = useComparisonStore.getState();

  const parts: string[] = [
    "You are a helpful database assistant embedded in Upsert, a cross-platform database comparison and migration tool.",
    "You can help with SQL queries, schema design, migration strategies, data transformation, and general database questions.",
    "Supported engines: SQL Server, PostgreSQL, MySQL, SQLite, MongoDB, Oracle, CosmosDB.",
    "",
    "IMPORTANT INSTRUCTIONS:",
    "- When asked about a table, column, or schema — look it up in the RELEVANT CONTEXT below and give a direct answer.",
    "- If information is truly not available in the context, say so clearly.",
  ];

  // ── RAG: retrieve relevant context chunks ────────────────────────────
  try {
    const results = await searchContext(userQuery, 5);
    if (results.length > 0) {
      parts.push("\n## Relevant Context (retrieved by similarity search)");
      for (const r of results) {
        parts.push(`\n### ${r.label} [${r.chunkType}] (score: ${r.score.toFixed(3)})`);
        parts.push(r.content);
      }
    }
  } catch {
    // RAG search failed — fall back to minimal context
  }

  // ── Lightweight inline state (always included, small) ────────────────
  if (connections.length > 0) {
    parts.push("\n## Connected Databases");
    for (const conn of connections) {
      const active = conn.id === activeConnectionId ? " (ACTIVE)" : "";
      const db = conn.database ? ` / ${conn.database}` : "";
      parts.push(`- ${conn.name}: ${conn.engine}${db} [${conn.status}]${active}`);
    }
  }

  // ── Migration state (full detail — only present when active) ────────
  if (migration.status !== "idle") {
    parts.push("\n## Active Migration");
    parts.push(`  Status: ${migration.status}`);
    parts.push(`  Mode: ${migration.config.mode}`);
    parts.push(`  Conflict Resolution: ${migration.config.conflictResolution}`);
    parts.push(`  Batch Size: ${migration.config.batchSize}`);

    if (migration.error) {
      parts.push(`  ERROR: ${migration.error}`);
    }

    if (migration.progress) {
      const p = migration.progress;
      parts.push(`  Progress: ${p.processedRows}/${p.totalRows} rows`);
      parts.push(`    Inserted: ${p.insertedRows}, Updated: ${p.updatedRows}, Deleted: ${p.deletedRows}, Skipped: ${p.skippedRows}`);
      if (p.errorCount > 0) {
        parts.push(`    ERRORS: ${p.errorCount}`);
      }
    }

    // Table-level errors (critical for diagnosing failures)
    if (migration.tableProgress.length > 0) {
      const failed = migration.tableProgress.filter(
        (tp) => tp.status === "failed" || tp.errors.length > 0,
      );
      if (failed.length > 0) {
        parts.push("  Table Errors:");
        for (const tp of failed) {
          parts.push(
            `    ${tp.tableName} [${tp.status}]: ${tp.processedRows}/${tp.totalRows} rows`,
          );
          for (const err of tp.errors.slice(0, 10)) {
            parts.push(
              `      - ${err.message}${err.rowIndex !== undefined ? ` (row ${err.rowIndex})` : ""}`,
            );
          }
          if (tp.errors.length > 10) {
            parts.push(`      ... and ${tp.errors.length - 10} more errors`);
          }
        }
      }
    }

    // Dry run results
    if (migration.dryRunResult) {
      const dr = migration.dryRunResult;
      parts.push("  Dry Run Summary:");
      for (const ts of dr.tableSummaries) {
        parts.push(
          `    ${ts.tableName}: +${ts.estimatedInserts} inserts, ~${ts.estimatedUpdates} updates, -${ts.estimatedDeletes} deletes, ${ts.estimatedSkips} skips`,
        );
      }
      if (dr.warnings.length > 0) {
        parts.push("  Dry Run Warnings:");
        for (const w of dr.warnings) {
          parts.push(`    - ${w}`);
        }
      }
      if (dr.errors.length > 0) {
        parts.push("  Dry Run Errors:");
        for (const e of dr.errors) {
          parts.push(`    - ${e}`);
        }
      }
    }

    // Table mappings
    if (migration.tableMappings.length > 0) {
      parts.push("  Table Mappings:");
      for (const m of migration.tableMappings.filter((m) => m.included)) {
        parts.push(
          `    ${m.sourceTable} → ${m.targetTable} (~${m.estimatedRows} rows)`,
        );
      }
    }

    // Transform rules
    if (migration.transformRules.length > 0) {
      parts.push("  Transform Rules:");
      for (const r of migration.transformRules) {
        parts.push(
          `    ${r.sourceColumn} → ${r.targetColumn} [${r.ruleType}]`,
        );
      }
    }
  }

  // ── Comparison results ──────────────────────────────────────────────
  if (comparison.schemaDiff) {
    const sd = comparison.schemaDiff;
    parts.push("\n## Schema Comparison Results");
    parts.push(`  Source: ${sd.sourceDatabase} → Target: ${sd.targetDatabase}`);
    parts.push(
      `  Summary: +${sd.summary.additions} added, -${sd.summary.removals} removed, ~${sd.summary.modifications} modified, ${sd.summary.unchanged} unchanged`,
    );
    if (sd.changes.length > 0) {
      parts.push("  Changes:");
      for (const c of sd.changes.slice(0, 30)) {
        parts.push(`    [${c.changeType}] ${c.objectType}: ${c.objectName}`);
        for (const d of c.details) {
          parts.push(
            `      ${d.property}: ${d.sourceValue ?? "(none)"} → ${d.targetValue ?? "(none)"}`,
          );
        }
      }
      if (sd.changes.length > 30) {
        parts.push(`    ... and ${sd.changes.length - 30} more changes`);
      }
    }
  }

  if (comparison.dataDiff) {
    const dd = comparison.dataDiff;
    parts.push("\n## Data Comparison Results");
    parts.push(`  ${dd.sourceTable} vs ${dd.targetTable}`);
    parts.push(
      `  Matched: ${dd.matchedRows}, To Insert: ${dd.insertedCount}, To Update: ${dd.updatedCount}, To Delete: ${dd.deletedCount}`,
    );
    if (dd.errorCount > 0) {
      parts.push(`  Errors: ${dd.errorCount}`);
    }
  }

  if (comparison.error) {
    parts.push(`\n## Comparison Error\n  ${comparison.error}`);
  }

  // Current view
  const activeTab = tabs.find((t) => t.id === activeTabId);
  if (activeTab) {
    parts.push(`\nUser is currently viewing: ${activeTab.type} - ${activeTab.title}`);
  }

  return parts.join("\n");
}
