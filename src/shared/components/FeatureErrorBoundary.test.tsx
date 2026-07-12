import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { FeatureErrorBoundary } from './FeatureErrorBoundary';

describe('FeatureErrorBoundary', () => {
  it('contains a feature crash and offers an explicit retry', () => {
    const original = console.error;
    console.error = vi.fn();
    let broken = true;
    function Feature() { if (broken) throw new Error('boom'); return <div>Recovered feature</div>; }
    render(<FeatureErrorBoundary feature="Editor"><Feature /></FeatureErrorBoundary>);
    expect(screen.getByRole('alert')).toHaveTextContent('Editor could not open');
    broken = false;
    fireEvent.click(screen.getByRole('button', { name: /try again/i }));
    expect(screen.getByText('Recovered feature')).toBeInTheDocument();
    console.error = original;
  });
});
