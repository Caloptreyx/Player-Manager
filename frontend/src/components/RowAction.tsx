import type { IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Button from '@/elements/buttons/Button.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { Access } from '../lib/access.ts';
import { useText } from './playerManager.ts';

/** Hidden when the user can never do it, disabled with the reason as tooltip while blocked. */
function useGate(access: Access) {
  const text = useText();
  if (!access.visible) return { hidden: true, reason: null };

  return { hidden: false, reason: access.blocker ? text(`blockers.${access.blocker}`, {}) : null };
}

export default function RowAction({
  icon,
  label,
  access,
  danger = false,
  onClick,
}: {
  icon: IconDefinition;
  label: string;
  access: Access;
  /** Turns red on hover. */
  danger?: boolean;
  onClick: () => void;
}) {
  const { hidden, reason } = useGate(access);
  if (hidden) return null;

  return (
    <Tooltip label={reason ?? label} multiline maw={260}>
      <ActionIcon
        size='md'
        variant='subtle'
        color='gray'
        className={
          danger ? 'hover:bg-(--mantine-color-red-light)! hover:text-(--mantine-color-red-light-color)!' : undefined
        }
        disabled={reason !== null}
        aria-label={label}
        onClick={onClick}
      >
        <FontAwesomeIcon icon={icon} />
      </ActionIcon>
    </Tooltip>
  );
}

export function GatedButton({
  icon,
  label,
  access,
  onClick,
}: {
  icon: IconDefinition;
  label: string;
  access: Access;
  onClick: () => void;
}) {
  const { hidden, reason } = useGate(access);
  if (hidden) return null;

  const button = (
    <Button size='xs' leftSection={<FontAwesomeIcon icon={icon} />} disabled={reason !== null} onClick={onClick}>
      {label}
    </Button>
  );

  return reason ? (
    <Tooltip label={reason} multiline maw={260}>
      {button}
    </Tooltip>
  ) : (
    button
  );
}
