import { memo, type ReactNode } from 'react';
import './TabletLayout.css';

interface TabletLayoutProps {
  topbar: ReactNode;
  content: ReactNode;
}

export const TabletLayout = memo(function TabletLayout({
  topbar,
  content,
}: TabletLayoutProps) {
  return (
    <div className="tablet-layout">
      <header className="layout-topbar">{topbar}</header>
      <main className="layout-content">{content}</main>
    </div>
  );
});
