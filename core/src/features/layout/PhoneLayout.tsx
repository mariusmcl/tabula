import { memo, type ReactNode } from 'react';
import './PhoneLayout.css';

interface PhoneLayoutProps {
  topbar: ReactNode;
  content: ReactNode;
  bottomNav?: ReactNode;
}

export const PhoneLayout = memo(function PhoneLayout({
  topbar,
  content,
  bottomNav,
}: PhoneLayoutProps) {
  return (
    <div className="phone-layout">
      <header className="layout-topbar">{topbar}</header>
      <main className="layout-content">{content}</main>
      {bottomNav && <nav className="layout-bottom-nav">{bottomNav}</nav>}
    </div>
  );
});
