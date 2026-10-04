import { type FormEvent, useState } from 'react';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Autocomplete from '@/elements/input/Autocomplete.tsx';
import Checkbox from '@/elements/input/Checkbox.tsx';
import Select from '@/elements/input/Select.tsx';
import TextInput from '@/elements/input/TextInput.tsx';
import { Modal, ModalFooter } from '@/elements/modals/Modal.tsx';
import { useTranslations } from '@/providers/TranslationProvider.tsx';
import { addBan, addOperator, addToWhitelist } from '../api.ts';
import { BEDROCK_LEVELS, type BedrockLevel, knownId, type PlayerRef, samePlayer, suggestions } from '../lib/players.ts';
import { type FieldError, validateId, validateName, validateReason } from '../lib/validation.ts';
import { useExtTranslations } from '../translations.ts';
import { usePlayerManager } from './playerManager.ts';

export type AddKind = 'whitelist' | 'operator' | 'ban';

const JAVA_LEVELS = ['1', '2', '3', '4'] as const;

// adds a player to the whitelist, operators or bans; only offers the options the current method can apply
export default function AddPlayerModal({
  kind,
  opened,
  onClose,
}: {
  kind: AddKind;
  opened: boolean;
  onClose: () => void;
}) {
  const { t } = useTranslations();
  const { t: tExt } = useExtTranslations();
  const { serverUuid, edition, overview, method, run } = usePlayerManager();

  const [name, setName] = useState('');
  const [id, setId] = useState('');
  const [reason, setReason] = useState('');
  const [javaLevel, setJavaLevel] = useState<string>('default');
  const [bedrockLevel, setBedrockLevel] = useState<BedrockLevel>('operator');
  const [limitFlag, setLimitFlag] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [loading, setLoading] = useState(false);

  const bedrock = edition === 'bedrock';
  // a running Java server gets console commands, which only take the name
  const fileMode = method === 'file';
  const existing: PlayerRef[] =
    kind === 'whitelist' ? overview.whitelist : kind === 'operator' ? overview.operators : overview.bans;

  const trimmedName = name.trim();
  const trimmedId = id.trim();
  const resolvedId = trimmedId || knownId(overview.known, trimmedName);
  // re-adding an operator in file mode updates its level, everything else would be a no-op
  const duplicateAllowed = kind === 'operator' && fileMode;

  const nameError: FieldError | null =
    validateName(edition, trimmedName) ??
    (!duplicateAllowed && existing.some((entry) => samePlayer(entry, { name: trimmedName, id: resolvedId }))
      ? 'duplicate'
      : null);
  const idError: FieldError | null =
    validateId(edition, trimmedId) ?? (kind === 'operator' && bedrock && !resolvedId ? 'xuidRequired' : null);
  const reasonError = kind === 'ban' ? validateReason(reason.trim()) : null;
  const invalid = nameError !== null || (fileMode && idError !== null) || reasonError !== null;

  const errorText = (error: FieldError | null) => (submitted && error ? tExt(`errors.${error}`, {}) : null);

  const close = () => {
    setName('');
    setId('');
    setReason('');
    setJavaLevel('default');
    setBedrockLevel('operator');
    setLimitFlag(false);
    setSubmitted(false);
    onClose();
  };

  const submit = async (e: FormEvent) => {
    e.preventDefault();
    setSubmitted(true);
    if (invalid) return;

    const playerId = fileMode ? (resolvedId ?? undefined) : undefined;
    const optionalReason = reason.trim() || undefined;

    setLoading(true);
    const ok =
      kind === 'whitelist'
        ? await run(
            () =>
              addToWhitelist(serverUuid, {
                name: trimmedName,
                id: playerId,
                ignores_player_limit: bedrock ? limitFlag : undefined,
              }),
            tExt(bedrock ? 'toast.allowlistAdded' : 'toast.whitelistAdded', { name: trimmedName }),
          )
        : kind === 'operator'
          ? await run(
              () =>
                addOperator(serverUuid, {
                  name: trimmedName,
                  id: playerId,
                  level: bedrock ? bedrockLevel : fileMode && javaLevel !== 'default' ? Number(javaLevel) : undefined,
                  bypasses_player_limit: !bedrock && fileMode ? limitFlag : undefined,
                }),
              tExt('toast.opped', { name: trimmedName }),
            )
          : await run(
              () => addBan(serverUuid, { name: trimmedName, id: playerId, reason: optionalReason }),
              tExt('toast.banned', { name: trimmedName }),
            );
    setLoading(false);
    if (ok) close();
  };

  const title =
    kind === 'whitelist'
      ? tExt(bedrock ? 'actions.allowlistAdd' : 'actions.whitelistAdd', {})
      : kind === 'operator'
        ? tExt('actions.addOperator', {})
        : tExt('actions.banPlayer', {});

  const knownPlayerId = trimmedId ? null : knownId(overview.known, trimmedName);

  return (
    <Modal title={title} opened={opened} onClose={() => !loading && close()}>
      <form onSubmit={submit} className='flex flex-col gap-3'>
        <Autocomplete
          label={tExt(bedrock ? 'form.gamertag' : 'form.name', {})}
          value={name}
          data={suggestions(overview.known, existing)}
          limit={20}
          error={errorText(nameError)}
          data-autofocus
          onChange={setName}
        />

        {fileMode && (
          <TextInput
            label={tExt(bedrock ? 'form.xuid' : 'form.uuid', {})}
            description={
              knownPlayerId
                ? tExt('form.knownId', { id: knownPlayerId })
                : kind === 'operator' && bedrock
                  ? tExt('form.xuidRequired', {})
                  : tExt('form.idOptional', {})
            }
            placeholder={knownPlayerId ?? undefined}
            value={id}
            error={errorText(idError)}
            onChange={(e) => setId(e.currentTarget.value)}
          />
        )}

        {kind === 'operator' && bedrock && (
          <Select
            label={tExt('form.level', {})}
            value={bedrockLevel}
            data={BEDROCK_LEVELS.map((level) => ({ value: level, label: tExt(`badges.${level}`, {}) }))}
            onChange={(value) => setBedrockLevel(BEDROCK_LEVELS.find((level) => level === value) ?? 'operator')}
          />
        )}

        {kind === 'operator' && !bedrock && fileMode && (
          <>
            <Select
              label={tExt('form.level', {})}
              value={javaLevel}
              data={[
                { value: 'default', label: tExt('form.levelDefault', {}) },
                ...JAVA_LEVELS.map((level) => ({ value: level, label: tExt(`form.level${level}`, {}) })),
              ]}
              onChange={(value) => setJavaLevel(value ?? 'default')}
            />
            <Checkbox
              label={tExt('form.bypassesLimit', {})}
              checked={limitFlag}
              onChange={(e) => setLimitFlag(e.currentTarget.checked)}
            />
          </>
        )}

        {kind === 'operator' && !bedrock && !fileMode && (
          <Alert color='blue' className='text-sm!'>
            {tExt('form.commandOptions', {})}
          </Alert>
        )}

        {kind === 'whitelist' && bedrock && (
          <Checkbox
            label={tExt('form.ignoresLimit', {})}
            checked={limitFlag}
            onChange={(e) => setLimitFlag(e.currentTarget.checked)}
          />
        )}

        {kind === 'ban' && (
          <TextInput
            label={tExt('form.reason', {})}
            placeholder={tExt('form.reasonPlaceholder', {})}
            value={reason}
            error={reasonError && tExt(`errors.${reasonError}`, {})}
            onChange={(e) => setReason(e.currentTarget.value)}
          />
        )}

        <ModalFooter>
          <Button type='submit' color={kind === 'ban' ? 'red' : undefined} loading={loading}>
            {kind === 'ban' ? tExt('actions.ban', {}) : tExt('form.add', {})}
          </Button>
          <Button variant='default' disabled={loading} onClick={close}>
            {t('common.button.cancel', {})}
          </Button>
        </ModalFooter>
      </form>
    </Modal>
  );
}
