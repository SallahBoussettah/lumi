import { Mic, Sparkles } from "lucide-react";

const mockConversations = [
  {
    id: "1",
    title: "Omniscient Architecture Sync",
    overview:
      "Discussed the new memory indexing latency improvements and ambient listening protocols...",
    timeAgo: "3h ago",
    accent: "purple" as const,
    sparkline: [1, 2, 1.5, 0.5, 1, 2.5, 1, 1.5],
  },
  {
    id: "2",
    title: "Evening Reflection",
    overview:
      "Personal notes on today's focus levels and evening wind-down routine optimization.",
    timeAgo: "5h ago",
    accent: "teal" as const,
    sparkline: [0.5, 1, 2.5, 2, 0.5, 1, 1.5, 2],
  },
  {
    id: "3",
    title: "Neural Interface Concept",
    overview:
      "Wild idea about mapping memory clusters to spatial coordinates in a virtual room...",
    timeAgo: "8h ago",
    accent: "amber" as const,
    sparkline: [2, 1.5, 0.5, 1, 1, 0.5, 2, 1],
  },
  {
    id: "4",
    title: "Product Review with Team",
    overview:
      "Action items: Fix the sidebar hover state and adjust the vertical divider opacity.",
    timeAgo: "10h ago",
    accent: "purple" as const,
    sparkline: [1, 0.5, 1, 1.5, 2.5, 1, 0.5, 1],
  },
  {
    id: "5",
    title: "Book Recommendation Extraction",
    overview:
      'Detected recommendation: "The Overstory" during casual chat with Marcus.',
    timeAgo: "14h ago",
    accent: "teal" as const,
    sparkline: [0.5, 0.5, 1, 1, 1, 0.5, 0.5, 0.5],
  },
];

const accentColors = {
  purple: "bg-brand-purple",
  teal: "bg-brand-teal",
  amber: "bg-brand-amber",
};

function getGreeting(): string {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
}

export function ConversationsPage() {
  return (
    <div className="px-10 py-10 max-w-[960px]">
      {/* Header */}
      <header className="flex justify-between items-start mb-12">
        <div>
          <h1 className="text-[22px] font-light text-text-primary tracking-tight mb-1">
            {getGreeting()}, Salah
          </h1>
          <p className="text-[13px] text-text-tertiary">
            You had 0 conversations today. 0 memories extracted.
          </p>
        </div>

        {/* Stat Circles */}
        <div className="relative w-36 h-[88px] flex-shrink-0">
          <div className="absolute top-0 left-1/2 -translate-x-1/2 flex flex-col items-center justify-center w-11 h-11 rounded-full border-[1.5px] border-brand-purple">
            <span className="text-[11px] font-bold text-white leading-none">0</span>
            <span className="text-[7px] uppercase tracking-tighter text-text-muted">convos</span>
          </div>
          <div className="absolute bottom-0 left-3 flex flex-col items-center justify-center w-11 h-11 rounded-full border-[1.5px] border-brand-teal">
            <span className="text-[11px] font-bold text-white leading-none">0</span>
            <span className="text-[7px] uppercase tracking-tighter text-text-muted">memories</span>
          </div>
          <div className="absolute bottom-0 right-3 flex flex-col items-center justify-center w-11 h-11 rounded-full border-[1.5px] border-brand-amber">
            <span className="text-[11px] font-bold text-white leading-none">0</span>
            <span className="text-[7px] uppercase tracking-tighter text-text-muted">tasks</span>
          </div>
        </div>
      </header>

      {/* Recent Section */}
      <section>
        <h2 className="text-[11px] font-bold text-text-ghost tracking-[0.3em] uppercase mb-5">
          RECENT
        </h2>

        <div>
          {mockConversations.map((conv, i) => (
            <div key={conv.id}>
              <div
                className={`group flex items-center py-4 px-3 rounded-lg transition-all duration-200 hover:bg-surface-hover cursor-pointer ${
                  i === 0 ? "bg-surface-active" : ""
                }`}
              >
                <div
                  className={`w-[3px] h-10 ${accentColors[conv.accent]} rounded-full mr-5 shrink-0`}
                />

                <div className="flex-1 min-w-0 mr-6">
                  <h3 className="text-[13px] font-medium text-white mb-1">
                    {conv.title}
                  </h3>
                  <p className="text-[12px] text-text-tertiary truncate">
                    {conv.overview}
                  </p>
                </div>

                <div className="flex items-center gap-5 shrink-0">
                  <div className="flex items-end gap-[2px] h-4">
                    {conv.sparkline.map((h, j) => (
                      <div
                        key={j}
                        className="w-[2px] bg-brand-purple/30 rounded-full"
                        style={{ height: `${h * 4}px` }}
                      />
                    ))}
                  </div>
                  <span className="text-[11px] text-text-muted whitespace-nowrap w-12 text-right">
                    {conv.timeAgo}
                  </span>
                </div>
              </div>
              {i < mockConversations.length - 1 && (
                <div className="h-px mx-3 bg-white/[0.04]" />
              )}
            </div>
          ))}
        </div>
      </section>

      {/* Bottom Insight Cards */}
      <div className="mt-12 flex gap-5">
        <div className="flex-[2] h-28 rounded-xl bg-surface-card border border-border-faint p-5 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <div className="w-10 h-10 rounded-full bg-brand-purple/15 flex items-center justify-center shrink-0">
              <Sparkles className="text-brand-purple" size={18} />
            </div>
            <div>
              <h4 className="text-white text-[12px] font-medium">
                Memory Consolidation
              </h4>
              <p className="text-[10px] text-text-muted uppercase tracking-wider mt-0.5">
                Waiting for data
              </p>
            </div>
          </div>
          <div className="w-28 h-1.5 bg-white/5 rounded-full overflow-hidden shrink-0">
            <div className="w-0 h-full bg-brand-purple rounded-full" />
          </div>
        </div>

        <div className="flex-1 h-28 rounded-xl bg-surface-card border border-border-faint p-5">
          <h4 className="text-white text-[12px] font-medium mb-3">
            Today's Focus
          </h4>
          <div className="flex gap-1 items-end h-12">
            {[40, 65, 30, 50, 75, 20].map((h, i) => (
              <div
                key={i}
                className="flex-1 rounded-sm bg-brand-purple"
                style={{
                  height: `${h}%`,
                  opacity: 0.15 + (h / 100) * 0.5,
                }}
              />
            ))}
          </div>
        </div>
      </div>

      {/* Floating Mic Button */}
      <button className="fixed bottom-6 right-6 w-12 h-12 rounded-full bg-brand-purple glow-purple text-white flex items-center justify-center transition-transform hover:scale-105 active:scale-95 z-50 cursor-pointer">
        <Mic size={22} />
      </button>
    </div>
  );
}
