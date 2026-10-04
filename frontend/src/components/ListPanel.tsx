import { faMagnifyingGlass } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import type { ReactNode } from 'react';
import TextInput from '@/elements/input/TextInput.tsx';
import { useExtTranslations } from '../translations.ts';

// toolbar (filter, info, actions) above a list of rows, with the empty and no-match states
export default function ListPanel({
  filter,
  onFilterChange,
  info,
  actions,
  total,
  emptyText,
  children,
}: {
  filter?: string;
  onFilterChange?: (filter: string) => void;
  info?: ReactNode;
  actions?: ReactNode;
  /** Number of entries before filtering. */
  total: number;
  emptyText: string;
  /** The filtered rows. */
  children: ReactNode[];
}) {
  const { t: tExt } = useExtTranslations();

  return (
    <div className='flex flex-col gap-2'>
      <div className='flex flex-wrap items-center gap-2'>
        {onFilterChange && total > 0 && (
          <TextInput
            size='xs'
            className='w-full sm:w-60'
            placeholder={tExt('lists.filter', {})}
            leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} />}
            value={filter}
            onChange={(e) => onFilterChange(e.currentTarget.value)}
          />
        )}
        <div className='min-w-0 flex-1 text-xs text-(--mantine-color-dimmed)'>{info}</div>
        {actions}
      </div>

      {total === 0 ? (
        <p className='py-6 text-center text-sm text-(--mantine-color-dimmed)'>{emptyText}</p>
      ) : children.length === 0 ? (
        <p className='py-6 text-center text-sm text-(--mantine-color-dimmed)'>{tExt('lists.noMatches', {})}</p>
      ) : (
        <div className='flex flex-col divide-y divide-(--mantine-color-default-border)'>{children}</div>
      )}
    </div>
  );
}
