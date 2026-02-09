const COMPLEX_KEYWORDS = [
  "explain",
  "design",
  "optimize",
  "migrate",
  "compare",
  "refactor",
  "architecture",
  "strategy",
  "best practice",
  "trade-off",
  "tradeoff",
  "performance",
  "normalization",
  "denormalization",
  "index strategy",
  "query plan",
  "execution plan",
];

const SQL_COMPLEX_PATTERNS = /\b(JOIN|SUBQUER|UNION|CTE|WITH\s+\w+\s+AS|PARTITION\s+BY|WINDOW|LATERAL|CROSS\s+APPLY)\b/i;
const CODE_BLOCK_PATTERN = /```[\s\S]*?```/;
const MULTI_TABLE_PATTERN = /\b(FROM|JOIN)\b.*\b(FROM|JOIN)\b/is;

export type QueryComplexity = "simple" | "complex";

export function classifyQuery(text: string): QueryComplexity {
  const trimmed = text.trim();

  // Long queries or multi-sentence are complex
  if (trimmed.length > 200) return "complex";

  const sentences = trimmed.split(/[.!?]+/).filter((s) => s.trim().length > 0);
  if (sentences.length > 2) return "complex";

  // Complex keyword match
  const lower = trimmed.toLowerCase();
  if (COMPLEX_KEYWORDS.some((kw) => lower.includes(kw))) return "complex";

  // SQL with JOINs/subqueries
  if (SQL_COMPLEX_PATTERNS.test(trimmed)) return "complex";

  // Contains code blocks
  if (CODE_BLOCK_PATTERN.test(trimmed)) return "complex";

  // References multiple tables
  if (MULTI_TABLE_PATTERN.test(trimmed)) return "complex";

  return "simple";
}

export function getModelForQuery(text: string): string {
  const complexity = classifyQuery(text);
  return complexity === "complex" ? "llama3.2:3b" : "tinyllama";
}
