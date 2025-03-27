// NavMenu.tsx
import React, { useEffect, useRef, useState } from 'react'
import LogoutIcon from '@/assets/icons/logout.svg'
import { useNavigate } from 'react-router-dom'
import { useVoteManagementContext } from '@/context/voteManagement'
//Icons
import { CaretRight, CalendarCheck, CheckFat, Notebook } from '@phosphor-icons/react'

interface NavMenuProps {}

const NAV_MENU_OPTIONS = [
  {
    name: 'Current Poll',
    icon: <CalendarCheck />,
    path: '/current',
  },
  {
    name: 'Historic Polls',
    icon: <CheckFat />,
    path: '/historic',
  },
  {
    name: 'About',
    icon: <Notebook />,
    path: '/about',
  },
]

const NavMenu: React.FC<NavMenuProps> = () => {
  const navigate = useNavigate()
  const { user, logout } = useVoteManagementContext()
  const menuRef = useRef<HTMLDivElement>(null)
  const [isOpen, setIsOpen] = useState<boolean>(false)
  const buttonRef = useRef<HTMLButtonElement>(null)

  const handleClickOutside = (event: MouseEvent) => {
    if (
      isOpen &&
      menuRef.current &&
      !menuRef.current.contains(event.target as Node) &&
      !buttonRef.current?.contains(event.target as Node)
    ) {
      setIsOpen(false)
    }
  }

  const toggleMenu = (event: React.MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation()
    setIsOpen(!isOpen)
  }

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside)
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
    }
  }, [isOpen])

  const handleNavigation = (path: string) => {
    navigate(path)
    return setIsOpen(!isOpen)
  }

  const handleLogout = () => {
    navigate('/')
    logout()
    return setIsOpen(!isOpen)
  }

  return user ? (
    <div className='relative'>
      <button
        ref={buttonRef}
        onClick={toggleMenu}
        className='flex items-center justify-between space-x-1 rounded-lg border-2 bg-white/60 px-2 py-1 duration-300 ease-in-out hover:bg-white'
      >
        <img src={user.pfpUrl} className='h-[20px] w-[20px] rounded-full' />
        <p className='text-xs font-bold'>@{user.username}</p>
        <CaretRight className={isOpen ? '-rotate-90 transition-transform duration-200' : ''} />
      </button>

      <div
        ref={menuRef}
        className={`absolute right-0 mt-4 w-40 transform rounded-lg border-2 border-slate-600/10 bg-white p-4  shadow-md ${
          isOpen ? 'scale-100' : 'scale-0'
        }`}
      >
        <div className='space-y-2'>
          {NAV_MENU_OPTIONS.map(({ name, path, icon }) => (
            <div key={name} className='flex cursor-pointer space-x-2 rounded p-1 hover:bg-gray-100' onClick={() => handleNavigation(path)}>
              {icon}
              <p className='block rounded-md text-sm font-semibold '>{name}</p>
            </div>
          ))}
          <div className='border-t-2'>
            <div className='mt-2 flex cursor-pointer space-x-2 rounded  p-1 hover:bg-gray-100' onClick={handleLogout}>
              <img src={LogoutIcon} />
              <p className='block rounded-md text-sm font-semibold '>Logout</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  ) : null
}

export default NavMenu
