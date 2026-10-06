import { faEllipsisVertical, type IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Fragment } from 'react';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Button from '@/elements/buttons/Button.tsx';
import Menu from '@/elements/overlays/Menu.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { Access } from '../lib/access.ts';
import type { PlayerAction } from './playerActions.ts';
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
  variant = 'filled',
  loading = false,
  onClick,
}: {
  icon: IconDefinition;
  label: string;
  access: Access;
  /** `default`: a secondary action next to a primary one. */
  variant?: 'filled' | 'default';
  loading?: boolean;
  onClick: () => void;
}) {
  const { hidden, reason } = useGate(access);
  if (hidden) return null;

  const button = (
    <Button
      variant={variant}
      leftSection={<FontAwesomeIcon icon={icon} />}
      disabled={reason !== null}
      loading={loading}
      onClick={onClick}
    >
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

/**
 * The overflow menu of a row; actions the user can never take are left out, blocked ones show why. `reserve`
 * keeps the space of an empty menu so the actions of table rows stay aligned.
 */
export function ActionMenu({ actions, reserve = false }: { actions: PlayerAction[]; reserve?: boolean }) {
  const text = useText();
  const items = actions.flatMap((action) => {
    if (!action.access.visible) return [];
    return [{ ...action, reason: action.access.blocker ? text(`blockers.${action.access.blocker}`, {}) : null }];
  });
  if (items.length === 0) return reserve ? <span aria-hidden className='inline-block w-7 shrink-0' /> : null;

  return (
    <Menu position='bottom-end' withinPortal shadow='md' width={240}>
      <Menu.Target>
        <ActionIcon size='md' variant='subtle' color='gray' aria-label={text('common.moreActions', {})}>
          <FontAwesomeIcon icon={faEllipsisVertical} />
        </ActionIcon>
      </Menu.Target>
      <Menu.Dropdown>
        {items.map((item, index) => (
          <Fragment key={item.key}>
            {item.danger && index > 0 && !items[index - 1].danger && <Menu.Divider />}
            <Menu.Item
              color={item.danger ? 'red' : undefined}
              leftSection={<FontAwesomeIcon icon={item.icon} fixedWidth />}
              disabled={item.reason !== null}
              onClick={item.onClick}
            >
              <span className='block'>{item.label}</span>
              {item.reason && (
                <span className='block text-xs leading-snug text-(--mantine-color-dimmed)'>{item.reason}</span>
              )}
            </Menu.Item>
          </Fragment>
        ))}
      </Menu.Dropdown>
    </Menu>
  );
}
