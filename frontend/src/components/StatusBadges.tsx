import Badge from '@/elements/data-display/Badge.tsx';
import type { ListKind } from '../lib/model.ts';
import type { PlayerStatus } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import { usePlayerManager, useText } from './playerManager.ts';

export function LevelBadge({ level }: { level: string }) {
  const { t: tExt } = useExtTranslations();
  const { ui } = usePlayerManager();

  return (
    <Badge size='sm' variant='light' color={ui.isOperatorLevel(level) ? 'violet' : 'gray'}>
      {ui.levelLabel(tExt, level, 'badge')}
    </Badge>
  );
}

// cross-references of a row with the other lists; `except` leaves out the list the row belongs to
export default function StatusBadges({ status, except }: { status: PlayerStatus; except?: ListKind }) {
  const text = useText();
  const operator = except === 'operators' ? undefined : status.operators;

  return (
    <>
      {operator &&
        (operator.level === null ? (
          <Badge size='sm' variant='light' color='violet'>
            {text('lists.operators.badge', {})}
          </Badge>
        ) : (
          <LevelBadge level={operator.level} />
        ))}
      {except !== 'whitelist' && status.whitelist && (
        <Badge size='sm' variant='light' color='green'>
          {text('lists.whitelist.badge', {})}
        </Badge>
      )}
      {except !== 'bans' && status.bans && (
        <Badge size='sm' variant='light' color='red'>
          {text('lists.bans.badge', {})}
        </Badge>
      )}
    </>
  );
}
