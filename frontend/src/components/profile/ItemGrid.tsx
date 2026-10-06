import { faCode, faTrashCan } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Tooltip as MantineTooltip } from '@mantine/core';
import classNames from 'classnames';
import { type ReactNode, useState } from 'react';
import { Modal } from '@/elements/modals/Modal.tsx';
import Menu from '@/elements/overlays/Menu.tsx';
import Code from '@/elements/typography/Code.tsx';
import { runProfileAction } from '../../api.ts';
import type { Item } from '../../lib/model.ts';
import { prettifyId, prettySnbt, romanNumeral } from '../../lib/profiles.ts';
import { usePlayerManager, useText } from '../playerManager.ts';
import ItemIcon from './ItemIcon.tsx';
import { useProfileView } from './profileView.ts';

// the inventory screens of the game: light grey panels of sunken slots, white counts with a drop shadow and the
// dark tooltip with the item name, enchantments and id; tailwind needs the full class names in the source

/** The look of the game's container screens; sizes the slots inside through `--slot`. */
export function ItemPanel({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div
      className={classNames(
        'w-fit max-w-full rounded-md border-2 border-t-white border-r-[#555] border-b-[#555] border-l-white bg-[#c6c6c6] p-2 [--slot:2.125rem] sm:p-3 sm:[--slot:2.75rem]',
        className,
      )}
    >
      {children}
    </div>
  );
}

const SLOT =
  'relative grid size-(--slot) shrink-0 place-items-center border-2 border-t-[#373737] border-r-white border-b-white border-l-[#373737] bg-[#8b8b8b]';

/** An item's display name: its custom name, else the words of its id. */
export const itemName = (item: Item) => item.name ?? prettifyId(item.id);

function ItemTooltip({ item }: { item: Item }) {
  const text = useText();
  const enchanted = item.enchantments.length > 0;

  return (
    <div className='flex max-w-72 flex-col gap-0.5 text-sm leading-snug'>
      <span
        className={classNames('font-medium', item.name && 'italic')}
        style={{ color: enchanted ? '#55ffff' : '#fff' }}
      >
        {itemName(item)}
      </span>
      {item.enchantments.map((enchantment) => (
        <span
          key={enchantment.id}
          style={{ color: enchantment.id.includes('curse') ? '#ff5555' : '#aaaaaa' }}
        >{`${prettifyId(enchantment.id)} ${romanNumeral(enchantment.level)}`}</span>
      ))}
      {item.count > 1 && <span style={{ color: '#aaaaaa' }}>{text('profile.item.count', { count: item.count })}</span>}
      {item.damage !== null && item.damage > 0 && (
        <span style={{ color: '#aaaaaa' }}>{text('profile.item.damage', { damage: item.damage })}</span>
      )}
      <span className='font-mono text-xs' style={{ color: '#7a7a7a' }}>
        {item.id}
      </span>
    </div>
  );
}

function NbtModal({ item, onClose }: { item: Item; onClose: () => void }) {
  const text = useText();

  return (
    <Modal
      opened
      size='lg'
      title={text('profile.item.nbtTitle', { item: itemName(item), slot: item.slot })}
      onClose={onClose}
    >
      <Code block className='max-h-[60vh] overflow-auto text-xs'>
        {prettySnbt(item.snbt)}
      </Code>
    </Modal>
  );
}

/** One slot; an item shows its tooltip on hover and opens its menu (remove, NBT) on click. */
export function ItemSlot({ item }: { item: Item | undefined }) {
  const text = useText();
  const { serverUuid, ui, confirm } = usePlayerManager();
  const { profile, label, editAccess } = useProfileView();
  const [nbt, setNbt] = useState(false);

  if (!item) return <div className={SLOT} aria-label={text('profile.item.empty', {})} />;

  const name = itemName(item);
  const removeReason = editAccess.blocker && text(`blockers.${editAccess.blocker}`, {});

  return (
    <>
      {nbt && <NbtModal item={item} onClose={() => setNbt(false)} />}
      <Menu position='bottom-start' withinPortal shadow='md' width={220}>
        <Menu.Target>
          <MantineTooltip
            label={<ItemTooltip item={item} />}
            position='top-start'
            openDelay={80}
            styles={{
              tooltip: {
                background: 'rgba(16, 0, 16, 0.94)',
                border: '2px solid #2a0a5c',
                borderRadius: 4,
                padding: '6px 9px',
              },
            }}
          >
            <button
              type='button'
              aria-label={name}
              className={classNames(
                SLOT,
                'cursor-pointer hover:bg-[#a0a0a0] focus-visible:outline-2 focus-visible:outline-white',
              )}
            >
              <ItemIcon id={item.id} src={ui.profile.itemIcon(item.id)} size='slot' />
              {item.enchantments.length > 0 && (
                <span aria-hidden className='pointer-events-none absolute inset-0 bg-[#9b5cff]/25 mix-blend-screen' />
              )}
              {item.count > 1 && (
                <span className='pointer-events-none absolute right-0.5 bottom-0 font-mono text-xs leading-none font-bold text-white [text-shadow:1px_1px_0_#3f3f3f] sm:text-sm'>
                  {item.count}
                </span>
              )}
            </button>
          </MantineTooltip>
        </Menu.Target>
        <Menu.Dropdown>
          <Menu.Label className='truncate'>{name}</Menu.Label>
          <Menu.Item leftSection={<FontAwesomeIcon icon={faCode} fixedWidth />} onClick={() => setNbt(true)}>
            {text('profile.item.nbt', {})}
          </Menu.Item>
          {editAccess.visible && (
            <Menu.Item
              color='red'
              leftSection={<FontAwesomeIcon icon={faTrashCan} fixedWidth />}
              disabled={removeReason !== null}
              onClick={() =>
                confirm({
                  title: text('profile.item.removeTitle', { item: name }),
                  content: text('profile.item.removeContent', {
                    item: item.count > 1 ? `${item.count} × ${name}` : name,
                    name: label,
                  }),
                  confirm: text('profile.item.remove', {}),
                  run: () => runProfileAction(serverUuid, profile.id, { action: 'clear_slot', slot: item.slot }),
                  success: text('profile.item.removed', { item: name }),
                })
              }
            >
              <span className='block'>{text('profile.item.remove', {})}</span>
              {removeReason && (
                <span className='block text-xs leading-snug text-(--mantine-color-dimmed)'>{removeReason}</span>
              )}
            </Menu.Item>
          )}
        </Menu.Dropdown>
      </Menu>
    </>
  );
}

/** Rows of nine slots, like the game's containers. */
export function SlotGrid({ slots, items }: { slots: readonly string[]; items: Map<string, Item> }) {
  return (
    <div className='grid w-fit grid-cols-9 gap-0.5'>
      {slots.map((slot) => (
        <ItemSlot key={slot} item={items.get(slot)} />
      ))}
    </div>
  );
}

/** Items in slots the grids have no place for, in a wrapping row. */
export function UnplacedItems({ items }: { items: Item[] }) {
  const text = useText();
  if (items.length === 0) return null;

  return (
    <div className='flex flex-col gap-1.5'>
      <span className='text-xs font-semibold text-[#3f3f3f]'>{text('profile.item.other', {})}</span>
      <div className='flex flex-wrap gap-0.5'>
        {items.map((item) => (
          <ItemSlot key={`${item.slot}:${item.id}`} item={item} />
        ))}
      </div>
    </div>
  );
}
