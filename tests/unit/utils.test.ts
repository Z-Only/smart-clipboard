import { describe, expect, it } from 'vitest';
import { cn } from '@/lib/utils';

describe('cn', () => {
  it('merges class names and resolves Tailwind conflicts', () => {
    expect(cn('px-2', 'px-4', 'text-sm', false && 'hidden')).toBe('px-4 text-sm');
  });
});
