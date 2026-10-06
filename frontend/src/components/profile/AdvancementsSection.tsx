import { faMagnifyingGlass, faTrophy } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useState } from 'react';
import Badge from '@/elements/data-display/Badge.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import { matchesFilter } from '../../lib/players.ts';
import { parseStoredDate, prettifyId } from '../../lib/profiles.ts';
import { useText } from '../playerManager.ts';
import { useProfileView } from './profileView.ts';

// the done advancements, newest first as the backend sends them, with the tab they belong to (`story/…`)
export default function AdvancementsSection() {
  const text = useText();
  const { profile } = useProfileView();
  const [filter, setFilter] = useState('');
  const { advancements } = profile;

  if (!advancements || advancements.items.length === 0) {
    return (
      <EmptyState
        flush
        icon={faTrophy}
        title={text('profile.advancements.noneTitle', {})}
        description={text('profile.advancements.none', {})}
      />
    );
  }

  const items = advancements.items
    .map((item) => {
      const path = item.id.slice(item.id.indexOf(':') + 1);
      const group = path.includes('/') ? prettifyId(path.slice(0, path.indexOf('/'))) : null;
      return { ...item, title: prettifyId(item.id), group, date: item.done_at && parseStoredDate(item.done_at) };
    })
    .filter((item) => matchesFilter([item.id, item.title, item.group], filter));

  return (
    <div className='flex flex-col gap-3'>
      <div className='flex flex-wrap items-center gap-x-3 gap-y-2'>
        <TextInput
          className='w-full sm:w-72'
          placeholder={text('profile.advancements.search', {})}
          aria-label={text('profile.advancements.search', {})}
          leftSection={<FontAwesomeIcon icon={faMagnifyingGlass} size='sm' />}
          value={filter}
          onChange={(e) => setFilter(e.currentTarget.value)}
        />
        <span className='text-sm font-medium'>{text('profile.advancements.done', { count: advancements.done })}</span>
      </div>

      {items.length === 0 ? (
        <p className='py-6 text-center text-sm text-(--mantine-color-dimmed)'>
          {text('lists.noMatches', { query: filter.trim() })}
        </p>
      ) : (
        <ul className='grid grid-cols-1 gap-2 md:grid-cols-2'>
          {items.map((item) => (
            <li
              key={item.id}
              className='flex items-center gap-3 rounded-md border border-(--mantine-color-default-border) p-2.5'
            >
              <span className='grid size-9 shrink-0 place-items-center rounded-md bg-[#f6c343]/15 text-[#e0a800]'>
                <FontAwesomeIcon icon={faTrophy} />
              </span>
              <span className='flex min-w-0 flex-1 flex-col'>
                <span className='truncate text-sm font-medium'>{item.title}</span>
                <span className='truncate font-mono text-xs text-(--mantine-color-dimmed)'>{item.id}</span>
              </span>
              <span className='flex shrink-0 flex-col items-end gap-1'>
                {item.group && (
                  <Badge size='xs' variant='light' color='gray'>
                    {item.group}
                  </Badge>
                )}
                {item.date && (
                  <FormattedTimestamp timestamp={item.date} className='text-xs text-(--mantine-color-dimmed)' />
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
