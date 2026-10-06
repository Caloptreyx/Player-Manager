import Tooltip from '@/elements/overlays/Tooltip.tsx';
import { banDate } from '../lib/lists.ts';
import type { Entry } from '../lib/model.ts';
import { useText } from './playerManager.ts';

// reason of a ban, then who issued it and when, as stored in the file; nothing for entries without them
export default function BanDetails({ entry }: { entry: Entry }) {
  const text = useText();

  const meta = [
    entry.source && text('lists.source', { source: entry.source }),
    entry.created && banDate(entry.created),
    entry.expires && text('lists.expires', { expires: banDate(entry.expires) }),
  ].filter((part) => part !== null && part !== '');
  if (!entry.reason && meta.length === 0) return null;

  const full = [entry.created, entry.expires].filter(Boolean).join(' → ');

  return (
    <div className='flex min-w-0 flex-col gap-0.5 whitespace-normal'>
      {entry.reason && <span className='text-sm break-words'>{entry.reason}</span>}
      {meta.length > 0 && (
        <Tooltip label={full} disabled={full === ''}>
          <span className='text-xs text-(--mantine-color-dimmed)'>{meta.join(' · ')}</span>
        </Tooltip>
      )}
    </div>
  );
}
