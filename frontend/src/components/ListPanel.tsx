import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import type { ReactNode } from 'react';
import TextInput from '@/elements/input/TextInput.tsx';
import { useText } from './playerManager.ts';

// toolbar (search, info, primary action) above the rows of a tab, with the empty and no-match states; an
// empty list without info shows only its empty state, which carries its own add button
export default function ListPanel({
  filter,
  onFilterChange,
  searchPlaceholder,
  info,
  actions,
  total,
  shown,
  empty,
  children,
}: {
  filter: string;
  onFilterChange: (filter: string) => void;
  /** Defaults to the search text of the lists (name, id or ip). */
  searchPlaceholder?: string;
  info?: ReactNode;
  actions?: ReactNode;
  /** Number of entries before filtering. */
  total: number;
  /** Number of entries after filtering. */
  shown: number;
  empty: ReactNode;
  /** The filtered rows. */
  children: ReactNode;
}) {
  const text = useText();
  const search = searchPlaceholder ?? text('lists.search', {});

  return (
    <div className='flex flex-col gap-3'>
      {(total > 0 || info) && (
        <div className='flex flex-wrap items-center gap-x-3 gap-y-2'>
          {total > 0 && (
            <TextInput
              // without info the search shares its row with the add button, even on small screens
              className={info ? 'w-full sm:w-72' : 'min-w-40 flex-1 sm:w-72 sm:flex-none'}
              placeholder={search}
              aria-label={search}
              leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} size='sm' />}
              value={filter}
              onChange={(e) => onFilterChange(e.currentTarget.value)}
            />
          )}
          {info && <div className='min-w-0 flex-1 text-sm text-(--mantine-color-dimmed)'>{info}</div>}
          <div className='ml-auto'>{actions}</div>
        </div>
      )}

      {total === 0 ? (
        empty
      ) : shown === 0 ? (
        <p className='py-8 text-center text-sm text-(--mantine-color-dimmed)'>
          {text('lists.noMatches', { query: filter.trim() })}{' '}
          <button
            type='button'
            className='cursor-pointer font-medium text-(--mantine-primary-color-filled) hover:underline'
            onClick={() => onFilterChange('')}
          >
            {text('lists.clearSearch', {})}
          </button>
        </p>
      ) : (
        children
      )}
    </div>
  );
}
