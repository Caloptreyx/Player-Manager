import { faBroom, faGift } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import { runProfileAction } from '../../api.ts';
import {
  ARMOR_SLOTS,
  ENDER_CHEST_SLOTS,
  HOTBAR_SLOTS,
  MAIN_SLOTS,
  OFFHAND_SLOT,
  slotItems,
} from '../../lib/profiles.ts';
import { usePlayerManager, useText } from '../playerManager.ts';
import { GatedButton } from '../RowAction.tsx';
import GiveItemModal from './GiveItemModal.tsx';
import { ItemPanel, ItemSlot, SlotGrid, UnplacedItems } from './ItemGrid.tsx';
import { useProfileView } from './profileView.ts';

/** Clears a whole container after a confirmation. */
function ClearButton({ container }: { container: 'inventory' | 'ender_chest' }) {
  const text = useText();
  const { serverUuid, confirm } = usePlayerManager();
  const { profile, label, editAccess } = useProfileView();
  const key = container === 'inventory' ? 'inventory' : 'enderChest';

  return (
    <GatedButton
      icon={faBroom}
      label={text(`profile.clear.${key}`, {})}
      variant='default'
      access={editAccess}
      onClick={() =>
        confirm({
          title: text(`profile.clear.${key}Title`, { name: label }),
          content: text(`profile.clear.${key}Content`, { name: label }),
          confirm: text('profile.clear.confirm', {}),
          run: () => runProfileAction(serverUuid, profile.id, { action: 'clear_container', container }),
          success: text(`profile.clear.${key}Done`, { name: label }),
        })
      }
    />
  );
}

// the inventory screen: armor beside the player, the offhand, three storage rows and the hotbar
export function InventorySection() {
  const { ui } = usePlayerManager();
  const text = useText();
  const { profile, giveAccess } = useProfileView();
  const [giving, setGiving] = useState(false);
  const { bySlot, unplaced } = slotItems(profile.inventory, 'inventory');
  const body = ui.profile.bodyUrl(profile, 256);

  return (
    <div className='flex flex-col gap-3'>
      <GiveItemModal opened={giving} onClose={() => setGiving(false)} />
      <div className='flex flex-wrap items-center justify-end gap-2'>
        <ClearButton container='inventory' />
        <GatedButton
          icon={faGift}
          label={text('profile.give.button', {})}
          access={giveAccess}
          onClick={() => setGiving(true)}
        />
      </div>

      <ItemPanel className='self-center'>
        <div className='flex flex-col gap-2 sm:gap-3'>
          <div className='flex items-end gap-0.5'>
            <div className='flex flex-col gap-0.5'>
              {ARMOR_SLOTS.map((slot) => (
                <ItemSlot key={slot} item={bySlot.get(slot)} />
              ))}
            </div>
            <div className='mx-1 grid h-[calc(var(--slot)*4+0.375rem)] w-[calc(var(--slot)*3)] place-items-center overflow-hidden border-2 border-t-[#373737] border-r-white border-b-white border-l-[#373737] bg-black'>
              {body && <img src={body} alt='' className='h-[92%] w-auto object-contain' draggable={false} />}
            </div>
            <ItemSlot item={bySlot.get(OFFHAND_SLOT)} />
          </div>
          <SlotGrid slots={MAIN_SLOTS} items={bySlot} />
          <SlotGrid slots={HOTBAR_SLOTS} items={bySlot} />
          <UnplacedItems items={unplaced} />
        </div>
      </ItemPanel>
    </div>
  );
}

export function EnderChestSection() {
  const text = useText();
  const { profile } = useProfileView();
  const { bySlot, unplaced } = slotItems(profile.ender_chest, 'ender_chest');

  return (
    <div className='flex flex-col gap-3'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <span className='inline-flex items-center gap-2 text-sm text-(--mantine-color-dimmed)'>
          {profile.ender_chest.length === 0 && text('profile.clear.empty', {})}
        </span>
        <ClearButton container='ender_chest' />
      </div>
      <ItemPanel className='self-center'>
        <div className='flex flex-col gap-2 sm:gap-3'>
          <SlotGrid slots={ENDER_CHEST_SLOTS} items={bySlot} />
          <UnplacedItems items={unplaced} />
        </div>
      </ItemPanel>
    </div>
  );
}
