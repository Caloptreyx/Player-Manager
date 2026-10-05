import type { Entry } from '../lib/model.ts';
import { useText } from './playerManager.ts';

// reason, source and dates of a ban, as stored in the file; nothing for entries without them
export default function BanDetails({ entry }: { entry: Entry }) {
  const text = useText();

  const parts = [
    entry.reason && text('lists.reason', { reason: entry.reason }),
    entry.source && text('lists.source', { source: entry.source }),
    entry.created && text('lists.created', { created: entry.created }),
    entry.expires && text('lists.expires', { expires: entry.expires }),
  ].filter((part) => part !== null && part !== '');

  return parts.length > 0 ? <span className='break-words'>{parts.join(' · ')}</span> : null;
}
