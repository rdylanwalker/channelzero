import * as React from 'react'
import { cn } from './utils'

export interface BadgeProps extends React.HTMLAttributes<HTMLSpanElement> {
  variant?: 'default' | 'secondary' | 'outline' | 'success' | 'warning' | 'destructive'
}

export function Badge({ className, variant = 'secondary', ...props }: BadgeProps) {
  const variants: Record<string, string> = {
    default: 'bg-accent/20 text-blue-300 border-accent/40',
    secondary: 'bg-muted text-muted-foreground border-border',
    outline: 'bg-transparent text-foreground border-border',
    success: 'bg-emerald-500/15 text-emerald-300 border-emerald-500/30',
    warning: 'bg-amber-500/15 text-amber-300 border-amber-500/30',
    destructive: 'bg-red-500/15 text-red-300 border-red-500/30',
  }
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium',
        variants[variant],
        className,
      )}
      {...props}
    />
  )
}
