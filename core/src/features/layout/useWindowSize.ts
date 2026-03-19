import { useState, useEffect } from 'react';

interface WindowSize {
  width: number;
  height: number;
}

interface UseWindowSizeReturn extends WindowSize {
  isPhone: boolean;
  isTablet: boolean;
  isDesktop: boolean;
}

const PHONE_BREAKPOINT = 768;
const TABLET_BREAKPOINT = 1024;

export function useWindowSize(): UseWindowSizeReturn {
  const [size, setSize] = useState<WindowSize>(() => ({
    width: typeof window !== 'undefined' ? window.innerWidth : 1024,
    height: typeof window !== 'undefined' ? window.innerHeight : 768,
  }));

  useEffect(() => {
    const handleResize = () => {
      setSize({
        width: window.innerWidth,
        height: window.innerHeight,
      });
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  const isPhone = size.width < PHONE_BREAKPOINT;
  const isTablet = size.width >= PHONE_BREAKPOINT && size.width < TABLET_BREAKPOINT;
  const isDesktop = size.width >= TABLET_BREAKPOINT;

  return {
    ...size,
    isPhone,
    isTablet,
    isDesktop,
  };
}
