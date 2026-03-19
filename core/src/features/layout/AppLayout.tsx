import { memo, type ReactNode } from 'react';
import { useWindowSize } from './useWindowSize';
import { DesktopLayout } from './DesktopLayout';
import { TabletLayout } from './TabletLayout';
import { PhoneLayout } from './PhoneLayout';

interface AppLayoutProps {
  topbar: ReactNode;
  content: ReactNode;
  bottomNav?: ReactNode;
}

export const AppLayout = memo(function AppLayout({
  topbar,
  content,
  bottomNav,
}: AppLayoutProps) {
  const { isPhone, isTablet } = useWindowSize();

  if (isPhone) {
    return (
      <PhoneLayout topbar={topbar} content={content} bottomNav={bottomNav} />
    );
  }

  if (isTablet) {
    return <TabletLayout topbar={topbar} content={content} />;
  }

  return <DesktopLayout topbar={topbar} content={content} />;
});
