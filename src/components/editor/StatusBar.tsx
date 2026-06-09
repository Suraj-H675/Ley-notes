interface StatusBarProps {
  content: string;
}

function countWords(text: string): number {
  return text
    .split(/\s+/)
    .filter((word) => word.length > 0).length;
}

function estimateReadingTime(wordCount: number): number {
  return Math.max(1, Math.ceil(wordCount / 200));
}

function formatNumber(n: number): string {
  return n.toLocaleString();
}

export function StatusBar({ content }: StatusBarProps) {
  const wordCount = countWords(content);
  const charCount = content.length;
  const readingTime = estimateReadingTime(wordCount);

  return (
    <div
      className="fixed bottom-0 left-0 right-0 flex items-center justify-center border-t border-border/40 py-0.5"
      style={{ height: 28 }}
    >
      <span className="text-[11px] text-muted-foreground/60">
        {formatNumber(wordCount)} words · {formatNumber(charCount)} characters · ~{readingTime} min read
      </span>
    </div>
  );
}