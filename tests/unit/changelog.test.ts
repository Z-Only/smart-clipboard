import { describe, expect, it } from 'vitest';
import { extractLocalizedNotes } from '@/lib/changelog';

describe('extractLocalizedNotes', () => {
  const dualLangNotes = [
    '<!-- lang:en -->',
    '### Added',
    '',
    '- **Quick Paste Panel**: Global shortcut invokes overlay',
    '',
    '### Changed',
    '',
    '- **Version bump to 2.8.0**',
    '',
    '<!-- lang:zh-CN -->',
    '### 新增',
    '',
    '- **快速粘贴面板**：全局快捷键唤出覆盖面板',
    '',
    '### 变更',
    '',
    '- **版本升级至 2.8.0**',
  ].join('\n');

  it('returns English block when locale is en', () => {
    const result = extractLocalizedNotes(dualLangNotes, 'en');
    expect(result).toContain('Quick Paste Panel');
    expect(result).not.toContain('快速粘贴面板');
  });

  it('returns Chinese block when locale is zh-CN', () => {
    const result = extractLocalizedNotes(dualLangNotes, 'zh-CN');
    expect(result).toContain('快速粘贴面板');
    expect(result).not.toContain('Quick Paste Panel');
  });

  it('falls back to English when requested locale is missing', () => {
    const enOnly = '<!-- lang:en -->\n### Added\n\n- Feature X';
    const result = extractLocalizedNotes(enOnly, 'zh-CN');
    expect(result).toContain('Feature X');
  });

  it('returns original string when no lang markers are present (backward compat)', () => {
    const legacy = 'Full Changelog: v2.7.0...v2.8.0';
    expect(extractLocalizedNotes(legacy, 'en')).toBe(legacy);
    expect(extractLocalizedNotes(legacy, 'zh-CN')).toBe(legacy);
  });

  it('returns empty string for null input', () => {
    expect(extractLocalizedNotes(null, 'en')).toBe('');
  });

  it('returns empty string for empty string input', () => {
    expect(extractLocalizedNotes('', 'en')).toBe('');
  });

  it('trims whitespace from extracted blocks', () => {
    const result = extractLocalizedNotes(dualLangNotes, 'en');
    expect(result).not.toMatch(/^\s/);
    expect(result).not.toMatch(/\s$/);
  });
});
