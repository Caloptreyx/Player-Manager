import type { Ban, IpBan } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';

// reason, source and dates of a ban, as stored in the file
export default function BanDetails({ ban }: { ban: Ban | IpBan }) {
  const { t: tExt } = useExtTranslations();

  const parts = [
    ban.reason && tExt('lists.reason', { reason: ban.reason }),
    ban.source && tExt('lists.source', { source: ban.source }),
    ban.created && tExt('lists.created', { created: ban.created }),
    ban.expires && tExt('lists.expires', { expires: ban.expires }),
  ].filter((part) => part !== null && part !== '');

  return parts.length > 0 ? <span className='break-words'>{parts.join(' · ')}</span> : null;
}
