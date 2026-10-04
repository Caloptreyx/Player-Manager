import { faUsers } from '@fortawesome/free-solid-svg-icons';
import { Extension, ExtensionContext } from 'shared';
import PlayerManagerPage from './pages/PlayerManagerPage.tsx';
import { getExtTranslations } from './translations.ts';

class CaloptreyxPlayerManagerExtension extends Extension {
  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes.addServerRoute({
        name: () => getExtTranslations().t('common.players', {}),
        icon: faUsers,
        path: '/players',
        element: PlayerManagerPage,
        permission: 'files.read-content',
      }),
    );
  }
}

export default new CaloptreyxPlayerManagerExtension();
