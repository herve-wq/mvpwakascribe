import { useState, useEffect, useMemo } from "react";
import { SearchBar } from "./SearchBar";
import { TranscriptionCard } from "./TranscriptionCard";
import { useTranscription } from "../../hooks/useTranscription";
import type { Transcription } from "../../lib/types";

interface HistoryProps {
  onClose: () => void;
  onSelectTranscription?: (transcription: Transcription) => void;
}

function groupByDate(transcriptions: Transcription[]) {
  const groups: { [key: string]: Transcription[] } = {};
  const today = new Date();
  today.setHours(0, 0, 0, 0);
  const yesterday = new Date(today);
  yesterday.setDate(yesterday.getDate() - 1);

  for (const t of transcriptions) {
    const date = new Date(t.createdAt);
    date.setHours(0, 0, 0, 0);

    let key: string;
    if (date.getTime() === today.getTime()) {
      key = "Aujourd'hui";
    } else if (date.getTime() === yesterday.getTime()) {
      key = "Hier";
    } else {
      key = date.toLocaleDateString("fr-FR", {
        weekday: "long",
        day: "numeric",
        month: "long",
      });
    }

    if (!groups[key]) {
      groups[key] = [];
    }
    groups[key].push(t);
  }

  return groups;
}

export function History({ onClose, onSelectTranscription }: HistoryProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const { transcriptions, loadTranscriptions, deleteTranscription } =
    useTranscription();

  useEffect(() => {
    loadTranscriptions();
  }, [loadTranscriptions]);

  const filteredTranscriptions = useMemo(() => {
    if (!searchQuery) return transcriptions;
    const query = searchQuery.toLowerCase();
    return transcriptions.filter(
      (t) =>
        t.rawText.toLowerCase().includes(query) ||
        t.sourceName?.toLowerCase().includes(query)
    );
  }, [transcriptions, searchQuery]);

  const groupedTranscriptions = useMemo(
    () => groupByDate(filteredTranscriptions),
    [filteredTranscriptions]
  );

  const handleDelete = async (id: string) => {
    if (confirm("Supprimer cette transcription ?")) {
      await deleteTranscription(id);
    }
  };

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="p-4 border-b border-[var(--color-border)] flex items-center justify-between">
        <h2 className="font-semibold text-[var(--color-text-primary)]">Historique</h2>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-[var(--color-bg-tertiary)]"
        >
          <svg
            className="w-5 h-5 text-[var(--color-text-muted)]"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>

      {/* Search */}
      <div className="p-4">
        <SearchBar
          value={searchQuery}
          onChange={setSearchQuery}
          placeholder="Rechercher dans l'historique..."
        />
      </div>

      {/* List */}
      <div className="flex-1 overflow-auto px-4 pb-4">
        {Object.keys(groupedTranscriptions).length === 0 ? (
          <div className="text-center py-8 text-[var(--color-text-muted)]">
            {searchQuery ? "Aucun resultat" : "Aucune transcription"}
          </div>
        ) : (
          <div className="space-y-4">
            {Object.entries(groupedTranscriptions).map(([date, items]) => (
              <div key={date}>
                <h3 className="text-xs font-medium text-[var(--color-text-muted)] uppercase tracking-wider mb-2">
                  {date}
                </h3>
                <div className="space-y-2">
                  {items.map((transcription) => (
                    <TranscriptionCard
                      key={transcription.id}
                      transcription={transcription}
                      onOpen={() => onSelectTranscription?.(transcription)}
                      onDelete={() => handleDelete(transcription.id)}
                    />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
