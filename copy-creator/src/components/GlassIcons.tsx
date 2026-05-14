import React, { useEffect, useState } from 'react';
import './GlassIcons.css';

export interface GlassIconsItem {
  icon: React.ReactElement;
  color: string;
  label: string;
  panelType?: string;
  customClass?: string;
}

export interface GlassIconsProps {
  items: GlassIconsItem[];
  className?: string;
  activePanelType?: string | null;
  onActiveChange?: (index: number | null) => void;
}

const GlassIcons: React.FC<GlassIconsProps> = ({
  items,
  className,
  activePanelType,
  onActiveChange,
}) => {
  const [activeIndex, setActiveIndex] = useState<number | null>(() => {
    if (activePanelType) {
      const idx = items.findIndex((item) => item.panelType === activePanelType);
      return idx >= 0 ? idx : 0;
    }
    return 0;
  });

  useEffect(() => {
    if (activePanelType) {
      const idx = items.findIndex((item) => item.panelType === activePanelType);
      setActiveIndex(idx >= 0 ? idx : null);
    } else {
      setActiveIndex(null);
    }
  }, [activePanelType, items]);

  const handleButtonClick = (index: number) => {
    if (activeIndex === index) return;
    setActiveIndex(index);
    onActiveChange?.(index);
  };

  return (
    <ul className={`wrapper ${className || ''}`}>
      {items.map((item, index) => (
        <li
          key={index}
          className={`icon-content ${item.customClass || ''}`}
          role="button"
          tabIndex={0}
          aria-label={item.label}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleButtonClick(index);
            }
          }}
        >
          <button
            className={`icon-btn ${activeIndex === index ? 'active' : ''}`}
            onClick={() => handleButtonClick(index)}
          >
            <div className="filled" />
            {item.icon}
          </button>
        </li>
      ))}
    </ul>
  );
};

export default GlassIcons;
