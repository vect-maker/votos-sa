import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      name: 'devices-list',
      component: () => import('@/views/DevicesListView.vue'),
      meta: {
        title: 'Devices List',
      },
    },
    {
      path: '/devices/:id',
      name: 'device-detail',
      component: () => import('@/views/DeviceDetailView.vue'),
      props: true,
      meta: {
        title: 'Device Telemetry & Controls',
      },
    },
    {
      path: '/:pathMatch(.*)*',
      redirect: { name: 'devices-list' },
    },
  ],
  scrollBehavior(_to, _from, savedPosition) {
    if (savedPosition) {
      return savedPosition
    }
    return { top: 0 }
  },
})

router.afterEach((to) => {
  const baseTitle = 'IoT Control Panel'
  if (to.meta.title && typeof to.meta.title === 'string') {
    document.title = `${to.meta.title} | ${baseTitle}`
  } else {
    document.title = baseTitle
  }
})

export default router
