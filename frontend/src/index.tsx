import { faUsers } from '@fortawesome/free-solid-svg-icons';
import { Extension, ExtensionContext } from 'shared';
import PlayerManagerPage from './pages/PlayerManagerPage.tsx';
import ProfilePage from './pages/ProfilePage.tsx';
import { getExtTranslations } from './translations.ts';

class CaloptreyxPlayerManagerExtension extends Extension {
  public initialize(ctx: ExtensionContext): void {
    ctx.extensionRegistry.enterRoutes((routes) =>
      routes
        .addServerRoute({
          name: () => getExtTranslations().t('common.players', {}),
          icon: faUsers,
          path: '/players',
          element: PlayerManagerPage,
          permission: 'files.read-content',
        })
        // a player's profile; unnamed sub-routes stay out of the sidebar and follow the visibility of `/players`
        .addServerRoute({
          name: undefined,
          path: '/players/:profileId',
          element: ProfilePage,
          permission: 'files.read-content',
        }),
    );
  }
}

export default new CaloptreyxPlayerManagerExtension();
