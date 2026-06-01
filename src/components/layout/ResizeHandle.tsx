import React, { useRef, useCallback, useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

interface ResizeHandleProps {
  onResize: (width: number) => void;
  defaultWidth: number;
}

export function ResizeHandle({ onResize, defaultWidth }: ResizeHandleProps) {
  const [isDragging, setIsDragging] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(defaultWidth);

  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      e.preventDefault();
      setIsDragging(true);
      startXRef.current = e.clientX;
      startWidthRef.current = defaultWidth;
    },
    [defaultWidth]
  );

  useEffect(() => {
    if (!isDragging) return;
    document.body.style.cursor = 'col-resize';

    const handleMouseMove = (e: MouseEvent) => {
      const delta = e.clientX - startXRef.current;
      onResize(startWidthRef.current + delta);
    };
    const handleMouseUp = () => setIsDragging(false);

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = '';
    };
  }, [isDragging, onResize]);

  return (
    <div
      onMouseDown={handleMouseDown}
      className={cn(
        'group absolute right-0 top-0 z-10 h-full w-1 cursor-col-resize',
        'transition-colors duration-150',
        'after:absolute after:right-0 after:top-1/2 after:h-8 after:w-px after:-translate-y-1/2',
        isDragging
          ? 'bg-primary after:bg-primary'
          : 'after:bg-transparent hover:after:bg-primary/60'
      )}
    />
  );
}
