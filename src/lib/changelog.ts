const LANG_MARKER_REGEX = /<!--\s*lang:(\S+)\s*-->/g;

export function extractLocalizedNotes(notes: string | null, locale: string): string {
  if (!notes) return '';

  const markers = [...notes.matchAll(LANG_MARKER_REGEX)];
  if (markers.length === 0) return notes;

  const blocks = new Map<string, string>();

  for (let i = 0; i < markers.length; i++) {
    const lang = markers[i][1];
    const startIndex = markers[i].index! + markers[i][0].length;
    const endIndex = i + 1 < markers.length ? markers[i + 1].index! : notes.length;
    blocks.set(lang, notes.slice(startIndex, endIndex).trim());
  }

  return blocks.get(locale) ?? blocks.get('en') ?? '';
}
