import { type FormEvent, useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Autocomplete from '@/elements/input/Autocomplete.tsx';
import NumberInput from '@/elements/input/NumberInput.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { prettifyId, seenItemIds } from '../../lib/profiles.ts';
import { GIVE_COUNT_MAX, normalizeItemId, validateItemId } from '../../lib/validation.ts';
import { usePlayerManager, useText } from '../playerManager.ts';
import ItemIcon from './ItemIcon.tsx';
import { useProfileView } from './profileView.ts';

// gives the online player an item with `give`; suggests the ids already in the profile, any id can be typed
export default function GiveItemModal({ opened, onClose }: { opened: boolean; onClose: () => void }) {
  const { t } = useTranslations();
  const text = useText();
  const { ui } = usePlayerManager();
  const { profile, label, act } = useProfileView();
  const [item, setItem] = useState('');
  const [count, setCount] = useState<number | string>(1);
  const [touched, setTouched] = useState(false);
  const [loading, setLoading] = useState(false);

  const itemError = validateItemId(item);
  const amount = typeof count === 'number' ? count : Number.parseInt(count, 10);
  const countValid = Number.isInteger(amount) && amount >= 1 && amount <= GIVE_COUNT_MAX;

  const close = () => {
    setItem('');
    setCount(1);
    setTouched(false);
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setTouched(true);
    const id = normalizeItemId(item);
    if (id === null || !countValid) return;

    setLoading(true);
    const ok = await act(
      { action: 'give', item: id, count: amount },
      text('profile.give.done', { count: amount, item: prettifyId(id), name: label }),
    );
    setLoading(false);
    if (ok) close();
  };

  return (
    <Modal title={text('profile.give.title', { name: label })} opened={opened} onClose={() => !loading && close()}>
      <form onSubmit={submit} className='flex flex-col gap-3'>
        <Autocomplete
          label={text('profile.give.item', {})}
          description={text('profile.give.itemDescription', {})}
          placeholder='minecraft:diamond'
          data={seenItemIds(profile)}
          value={item}
          onChange={setItem}
          error={touched && itemError ? text(`errors.${itemError}`, {}) : undefined}
          renderOption={({ option }) => (
            <span className='flex items-center gap-2'>
              <ItemIcon id={option.value} src={ui.profile.itemIcon(option.value)} size={20} />
              <span className='flex min-w-0 flex-col'>
                <span className='truncate text-sm'>{prettifyId(option.value)}</span>
                <span className='truncate font-mono text-xs text-(--mantine-color-dimmed)'>{option.value}</span>
              </span>
            </span>
          )}
          data-autofocus
        />
        <NumberInput
          label={text('profile.give.count', {})}
          min={1}
          max={GIVE_COUNT_MAX}
          clampBehavior='strict'
          allowDecimal={false}
          value={count}
          onChange={setCount}
          error={touched && !countValid ? `1 – ${GIVE_COUNT_MAX}` : undefined}
        />

        <ModalFooter>
          <Button type='submit' loading={loading}>
            {text('profile.give.submit', {})}
          </Button>
          <Button variant='default' disabled={loading} onClick={close}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </form>
    </Modal>
  );
}
