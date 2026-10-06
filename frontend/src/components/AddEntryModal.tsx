import { faChevronDown, faCircleInfo } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Input } from '@mantine/core';
import classNames from 'classnames';
import { type FormEvent, useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Autocomplete from '@/elements/input/Autocomplete.tsx';
import Select from '@/elements/input/Select.tsx';
import Switch from '@/elements/input/Switch.tsx';
import TextArea from '@/elements/input/TextArea.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import Collapse from '@/elements/layout/Collapse.tsx';
import SegmentedControl from '@/elements/layout/SegmentedControl.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { addToList } from '../api.ts';
import { addBody, isDuplicate } from '../lib/lists.ts';
import type { ListSpec } from '../lib/model.ts';
import { knownId, sameName, suggestions } from '../lib/players.ts';
import {
  type FieldError,
  REASON_MAX_LENGTH,
  validateId,
  validateIp,
  validateName,
  validateReason,
} from '../lib/validation.ts';
import { useExtTranslations } from '../translations.ts';
import { LIST_STYLES } from './listStyles.ts';
import { usePlayerManager, useText } from './playerManager.ts';

/** Up to this many levels fit a segmented control; more get a select. */
const SEGMENTED_MAX = 4;

// adds an entry to a list; only offers the fields the list spec accepts in the current mode
export default function AddEntryModal({
  spec,
  opened,
  onClose,
}: {
  spec: ListSpec;
  opened: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const text = useText();
  const { serverUuid, game, ui, overview, run } = usePlayerManager();

  const [name, setName] = useState('');
  const [id, setId] = useState('');
  const [ip, setIp] = useState('');
  const [reason, setReason] = useState('');
  // null: the level the spec defaults to, which may change with the server state while the form is open
  const [level, setLevel] = useState<string | null>(null);
  const [limitFlag, setLimitFlag] = useState(false);
  const [advanced, setAdvanced] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [loading, setLoading] = useState(false);

  const { kind, levels } = spec;
  const entries = overview.lists[kind] ?? [];
  const byIp = spec.target === 'ip';
  const withId = spec.id === 'optional';

  const trimmedName = name.trim();
  const trimmedId = id.trim();
  const trimmedIp = ip.trim();
  const trimmedReason = reason.trim();
  const knownPlayerId = byIp || trimmedId ? null : knownId(overview.known, trimmedName);
  const resolvedId = trimmedId || knownPlayerId;
  const duplicate = isDuplicate(spec, entries, { name: trimmedName, id: resolvedId, ip: trimmedIp });

  const nameError: FieldError | null = byIp
    ? null
    : (validateName(game, trimmedName) ?? (duplicate ? 'duplicate' : null));
  const idError = withId ? validateId(game, trimmedId) : null;
  const ipError: FieldError | null = byIp ? (validateIp(trimmedIp) ?? (duplicate ? 'duplicate' : null)) : null;
  const reasonError = spec.reason ? validateReason(trimmedReason) : null;
  const invalid = nameError !== null || idError !== null || ipError !== null || reasonError !== null;

  const errorText = (error: FieldError | null) => (submitted && error ? text(`errors.${error}`, {}) : null);
  const note = ui.addFormNote(tExt, spec);
  // few levels show as segments with their short label and the long one underneath, many as a select
  const segmented = levels !== null && levels.options.length <= SEGMENTED_MAX;
  const selectedLevel = levels ? (level ?? levels.default) : null;
  const levelOption = selectedLevel === null ? null : ui.levelLabel(tExt, selectedLevel, 'option');
  const levelHint =
    segmented && selectedLevel !== null && levelOption !== ui.levelLabel(tExt, selectedLevel, 'badge')
      ? levelOption
      : null;

  const close = () => {
    setName('');
    setId('');
    setIp('');
    setReason('');
    setLevel(null);
    setLimitFlag(false);
    setAdvanced(false);
    setSubmitted(false);
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSubmitted(true);
    if (idError) setAdvanced(true);
    if (invalid) return;

    const body = addBody(spec, {
      name: trimmedName,
      id: resolvedId,
      ip: trimmedIp,
      level: level ?? undefined,
      bypasses_player_limit: limitFlag,
      reason: trimmedReason,
    });
    setLoading(true);
    const ok = await run(
      () => addToList(serverUuid, kind, body),
      text(`lists.${kind}.added`, { name: byIp ? trimmedIp : trimmedName }),
    );
    setLoading(false);
    if (ok) close();
  };

  return (
    <Modal title={text(`lists.${kind}.addTitle`, {})} opened={opened} onClose={() => !loading && close()}>
      <form onSubmit={submit} className='flex flex-col gap-4'>
        {byIp ? (
          <TextInput
            label={text('form.ip', {})}
            placeholder='203.0.113.7'
            value={ip}
            error={errorText(ipError)}
            data-autofocus
            onChange={(e) => setIp(e.currentTarget.value)}
          />
        ) : (
          <Autocomplete
            label={text('form.name', {})}
            value={name}
            data={suggestions(overview.known, entries)}
            limit={20}
            error={errorText(nameError)}
            data-autofocus
            leftSection={
              overview.known.some((player) => sameName(player.name, trimmedName)) ? (
                <Avatar
                  size={20}
                  radius='sm'
                  src={ui.avatarUrl({ name: trimmedName, id: resolvedId }, 40)}
                  name={trimmedName}
                />
              ) : undefined
            }
            renderOption={({ option }) => (
              <div className='flex items-center gap-2'>
                <Avatar
                  size={24}
                  radius='sm'
                  src={ui.avatarUrl({ name: option.value, id: knownId(overview.known, option.value) }, 48)}
                  name={option.value}
                />
                <span className='text-sm'>{option.value}</span>
              </div>
            )}
            onChange={setName}
          />
        )}

        {levels && selectedLevel !== null && (
          <Input.Wrapper
            label={text('form.level', {})}
            description={levelHint}
            inputWrapperOrder={['label', 'input', 'description']}
          >
            {segmented ? (
              <SegmentedControl
                fullWidth
                value={selectedLevel}
                data={levels.options.map((option) => ({ value: option, label: ui.levelLabel(tExt, option, 'badge') }))}
                onChange={setLevel}
              />
            ) : (
              <Select
                value={selectedLevel}
                data={levels.options.map((option) => ({ value: option, label: ui.levelLabel(tExt, option, 'option') }))}
                onChange={setLevel}
              />
            )}
          </Input.Wrapper>
        )}

        {spec.bypasses_player_limit && (
          <Switch
            label={text('badges.bypassesLimit', {})}
            description={text('form.bypassesLimit', {})}
            checked={limitFlag}
            onChange={(e) => setLimitFlag(e.currentTarget.checked)}
          />
        )}

        {spec.reason && (
          <TextArea
            label={text('form.reason', {})}
            placeholder={text('form.reasonPlaceholder', {})}
            autosize
            minRows={2}
            maxRows={5}
            value={reason}
            description={text('form.reasonCount', { count: [...trimmedReason].length, max: REASON_MAX_LENGTH })}
            inputWrapperOrder={['label', 'input', 'description', 'error']}
            error={reasonError && text(`errors.${reasonError}`, {})}
            onKeyDown={(e) => {
              // reasons are one line: Enter submits like in the other fields
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                e.currentTarget.form?.requestSubmit();
              }
            }}
            onChange={(e) => setReason(e.currentTarget.value)}
          />
        )}

        {withId && (
          <div className='flex flex-col gap-2'>
            <button
              type='button'
              aria-expanded={advanced}
              className='flex w-fit cursor-pointer items-center gap-1.5 text-sm! text-(--mantine-color-dimmed) hover:text-(--mantine-color-text)'
              onClick={() => setAdvanced((open) => !open)}
            >
              <FontAwesomeIcon
                icon={faChevronDown}
                size='xs'
                className={classNames('transition-transform', !advanced && '-rotate-90')}
              />
              {text('form.advanced', {})}
            </button>
            <Collapse expanded={advanced}>
              <TextInput
                label={text('form.id', {})}
                description={knownPlayerId ? text('form.knownId', { id: knownPlayerId }) : text('form.idOptional', {})}
                placeholder={knownPlayerId ?? undefined}
                value={id}
                error={errorText(idError)}
                onChange={(e) => setId(e.currentTarget.value)}
              />
            </Collapse>
          </div>
        )}

        {note && (
          <p className='flex items-start gap-2 text-xs text-(--mantine-color-dimmed)'>
            <FontAwesomeIcon icon={faCircleInfo} className='mt-0.5' />
            {note}
          </p>
        )}

        <ModalFooter>
          <Button type='submit' color={LIST_STYLES[kind].danger ? 'red' : undefined} loading={loading}>
            {text(`lists.${kind}.submit`, {})}
          </Button>
          <Button variant='default' disabled={loading} onClick={close}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </form>
    </Modal>
  );
}
