import React, { useRef, useCallback, useEffect, useState } from 'react';

interface ResizeHandleProps {
  onResize: (width: number) => void;
  defaultWidth: number;
}

export function ResizeHandle({ onResize, defaultWidth }: ResizeHandleProps) {
  const [isDragging, setIsDragging] = useState(false);
  const startXRef = useRef(0);
  const startWidthRef = useRef(defaultWidth);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsDragging(true);
    startXRef.current = e.clientX;
    startWidthRef.current = defaultWidth;
  }, [defaultWidth]);

  useEffect(() => {
    if (!isDragging) return;

    const handleMouseMove = (e: MouseEvent) => {
      const delta = e.clientX - startXRef.current;
      const newWidth = startWidthRef.current + delta;
      onResize(newWidth);
    };

    const handleMouseUp = () => {
      setIsDragging(false);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);

    return () => {
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };
  }, [isDragging, onResize]);

  return (
    <div
      className={`absolute right-0 top-0 h-full w-1 cursor-col-resize transition-colors ${
        isDragging ? 'bg-primary' : 'hover:bg-primary/50'
      }`}
      onMouseDown={handleMouseDown}
    />
  );
}
