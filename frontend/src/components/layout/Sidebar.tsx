import React from 'react'
import Link from 'next/link'

const Sidebar = () => {
  const menuItems = [
    { label: 'Dashboard', path: '/' },
    { label: 'Trading', path: '/trading' },
    { label: 'Backtest', path: '/backtest' },
    { label: 'Settings', path: '/settings' },
  ]

  return (
    <aside className="w-64 h-full bg-slate-900 text-white p-4">
      <nav>
        <ul className="space-y-2">
          {menuItems.map((item) => (
            <li key={item.path}>
              <Link 
                href={item.path}
                className="block px-4 py-2 rounded hover:bg-slate-700 transition-colors"
              >
                {item.label}
              </Link>
            </li>
          ))}
        </ul>
      </nav>
    </aside>
  )
}

export default Sidebar