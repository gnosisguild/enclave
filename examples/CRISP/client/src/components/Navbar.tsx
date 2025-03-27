import React from 'react'
import Logo from '@/assets/icons/logo.svg'
import { Link } from 'react-router-dom'
import NavMenu from '@/components/NavMenu'
import { useVoteManagementContext } from '@/context/voteManagement'

const PAGES = [
  {
    label: 'About',
    path: '/about',
  },
  {
    label: 'Historic Polls',
    path: '/historic',
  },
]

const Navbar: React.FC = () => {
  const { user } = useVoteManagementContext()
  return (
    <nav className='absolute left-0 top-0 z-10 w-screen px-6 lg:px-9'>
      <div className='mx-auto max-w-screen-xl'>
        <div className='flex h-20 items-center justify-between'>
          <Link
            to={'/'}
            className='hover:text-twilight-blue-600 cursor-pointer font-bold text-slate-600 duration-300 ease-in-out hover:opacity-70'
          >
            <img src={Logo} alt='CRISP Logo' className='h-6 cursor-pointer duration-300 ease-in-out hover:opacity-70 md:h-8' />
          </Link>

          <div className='flex items-center gap-8'>
            {PAGES.map(({ label, path }) => (
              <Link
                key={label}
                to={path}
                className='hover:text-twilight-blue-600 cursor-pointer font-bold text-slate-600 duration-300 ease-in-out hover:opacity-70 max-md:hidden'
              >
                {label}
              </Link>
            ))}
            {!user && (
              <Link
                to={PAGES[1].path}
                className='hover:text-twilight-blue-600 cursor-pointer font-bold text-slate-600 duration-300 ease-in-out hover:opacity-70 md:hidden'
              >
                {PAGES[1].label}
              </Link>
            )}
            <NavMenu />
          </div>
        </div>
      </div>
    </nav>
  )
}

export default Navbar
