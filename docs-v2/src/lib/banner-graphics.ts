export type BannerGraphic = {
  id: string
  src: string
  title: string
  description: string
  artist: string
  weight: number
  objectPosition: `${number}% ${number}%`
}

export const BANNER_GRAPHICS = [
  {
    id: 'graphic-01',
    src: '/banners/graphic-01.png',
    title: 'Open water study',
    description: 'A pale watercolor study of rippling open water.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '50% 55%',
  },
  {
    id: 'graphic-02',
    src: '/banners/graphic-02.png',
    title: 'Shore landing',
    description: 'A figure hauling a small boat across a rocky shore.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '48% 46%',
  },
  {
    id: 'graphic-03',
    src: '/banners/graphic-03.png',
    title: 'Sailboat at anchor',
    description: 'A white sailboat sitting low in blue water.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '58% 52%',
  },
  {
    id: 'graphic-04',
    src: '/banners/graphic-04.png',
    title: 'Blue water study',
    description: 'A soft watercolor view of blue water and white foam.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '50% 55%',
  },
  {
    id: 'graphic-05',
    src: '/banners/graphic-05.png',
    title: 'Boat on the beach',
    description: 'A small wooden boat pulled onto a rocky beach.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '48% 46%',
  },
  {
    id: 'graphic-06',
    src: '/banners/graphic-06.png',
    title: 'Harbor sail',
    description: 'A moored sailboat under a bright, clouded sky.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '58% 52%',
  },
  {
    id: 'graphic-07',
    src: '/banners/graphic-07.png',
    title: 'Boatman offshore',
    description: 'A boatman standing in a small craft on open water.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '52% 48%',
  },
  {
    id: 'graphic-08',
    src: '/banners/graphic-08.png',
    title: 'Weathered hull',
    description: 'A long boat resting against dark rocks and shallow water.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '50% 46%',
  },
  {
    id: 'graphic-09',
    src: '/banners/graphic-09.png',
    title: 'Boats in harbor',
    description: 'Small working boats gathered in a bright harbor.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '52% 48%',
  },
  {
    id: 'graphic-10',
    src: '/banners/graphic-10.png',
    title: 'White boat study',
    description: 'A white boat with figures set against turquoise water.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '58% 50%',
  },
  {
    id: 'graphic-11',
    src: '/banners/graphic-11.png',
    title: 'Leaping fish',
    description: 'A large fish suspended above a small boat at sea.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '50% 38%',
  },
  {
    id: 'graphic-12',
    src: '/banners/graphic-12.png',
    title: 'Dark surf',
    description: 'Waves breaking over dark rocks under a heavy sky.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '50% 50%',
  },
  {
    id: 'graphic-13',
    src: '/banners/graphic-13.png',
    title: 'Breaking wave',
    description: 'A wave breaking hard against black coastal rocks.',
    artist: 'CC0 artwork',
    weight: 1,
    objectPosition: '50% 50%',
  },
] satisfies BannerGraphic[]

export function getBannerGraphic(src: string): BannerGraphic {
  return (
    BANNER_GRAPHICS.find((graphic) => graphic.src === src) ?? {
      id: src,
      src,
      title: 'Banner artwork',
      description: 'Decorative banner artwork.',
      artist: 'CC0 artwork',
      weight: 1,
      objectPosition: '50% 50%',
    }
  )
}
