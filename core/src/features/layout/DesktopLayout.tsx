import { memo, type ReactNode } from 'react';
import './DesktopLayout.css';

interface DesktopLayoutProps {
  topbar: ReactNode;
  content: ReactNode;
}

export const DesktopLayout = memo(function DesktopLayout({
  topbar,
  content,
}: DesktopLayoutProps) {
  return (
    <div className="desktop-layout">
      <header className="layout-topbar">{topbar}</header>
      <main className="layout-content">{content}</main>
    </div>
  );
});
