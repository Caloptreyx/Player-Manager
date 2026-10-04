import Badge from '@/elements/data-display/Badge.tsx';
import type { OperatorLevel, PlayerStatus } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import { usePlayerManager } from './playerManager.ts';

export function LevelBadge({ level }: { level: OperatorLevel }) {
  const { t: tExt } = useExtTranslations();

  return (
    <Badge size='xs' variant='light' color={level === 'member' || level === 'visitor' ? 'gray' : 'violet'}>
      {typeof level === 'number' ? tExt('badges.opLevel', { level }) : tExt(`badges.${level}`, {})}
    </Badge>
  );
}

// cross-references of a row with the other lists; `except` leaves out the list the row belongs to
export default function StatusBadges({
  status,
  except,
}: {
  status: PlayerStatus;
  except?: 'operator' | 'whitelisted' | 'banned';
}) {
  const { t: tExt } = useExtTranslations();
  const { edition } = usePlayerManager();

  return (
    <>
      {except !== 'operator' && status.operator && <LevelBadge level={status.operator.level} />}
      {except !== 'whitelisted' && status.whitelisted && (
        <Badge size='xs' variant='light' color='green'>
          {tExt(edition === 'bedrock' ? 'badges.allowlisted' : 'badges.whitelisted', {})}
        </Badge>
      )}
      {except !== 'banned' && status.banned && (
        <Badge size='xs' variant='light' color='red'>
          {tExt('badges.banned', {})}
        </Badge>
      )}
    </>
  );
}
