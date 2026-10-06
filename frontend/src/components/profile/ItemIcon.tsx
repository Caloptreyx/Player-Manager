import classNames from 'classnames';
import { useState } from 'react';
import { idInitials } from '../../lib/profiles.ts';

// the rendered icon of an item, block or mob (pixel art, so scaled without smoothing); ids the icon source does
// not know (modded items, ids newer than it) get a tile with the initials of the id instead. `slot` sizes it to
// the `--slot` of the surrounding item panel
export default function ItemIcon({ id, src, size }: { id: string; src: string | null; size: number | 'slot' }) {
  const [failed, setFailed] = useState<string | null>(null);
  const slot = size === 'slot';
  const style = slot ? undefined : { width: size, height: size };

  if (src === null || failed === src) {
    return (
      <span
        aria-hidden
        className={classNames(
          'grid shrink-0 place-items-center rounded-sm bg-black/30 font-mono leading-none font-bold text-white/90 select-none',
          slot ? 'size-[calc(var(--slot)-0.75rem)] text-[length:calc(var(--slot)*0.3)]' : 'text-[10px]',
        )}
        style={style}
      >
        {idInitials(id)}
      </span>
    );
  }

  return (
    <img
      src={src}
      alt=''
      loading='lazy'
      draggable={false}
      className={classNames('shrink-0 object-contain select-none', slot && 'size-[calc(var(--slot)-0.75rem)]')}
      style={{ ...style, imageRendering: 'pixelated' }}
      onError={() => setFailed(src)}
    />
  );
}
