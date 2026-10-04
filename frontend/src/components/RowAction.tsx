import type { IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Button from '@/elements/buttons/Button.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { Access } from '../lib/access.ts';
import { useExtTranslations } from '../translations.ts';

/** Hidden when the user can never do it, disabled with the reason as tooltip while blocked. */
function useGate(access: Access | undefined, disabledReason: string | null | undefined) {
  const { t: tExt } = useExtTranslations();
  if (access && !access.visible) return { hidden: true, reason: null };

  return {
    hidden: false,
    reason: disabledReason ?? (access?.blocker ? tExt(`blockers.${access.blocker}`, {}) : null),
  };
}

export default function RowAction({
  icon,
  label,
  access,
  disabledReason,
  danger = false,
  onClick,
}: {
  icon: IconDefinition;
  label: string;
  access?: Access;
  /** Disables the action for a reason specific to this row. */
  disabledReason?: string | null;
  /** Turns red on hover. */
  danger?: boolean;
  onClick: () => void;
}) {
  const { hidden, reason } = useGate(access, disabledReason);
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
  const { hidden, reason } = useGate(access, null);
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
